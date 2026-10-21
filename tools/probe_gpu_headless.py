#!/usr/bin/env python3
"""Bounded by caller. Render one offscreen pixel; never acquire DRM master or present."""
import ctypes as C
import json
import os


def main():
    p=C.c_void_p; i=C.c_int; u=C.c_uint
    libs={name:C.CDLL(name) for name in ['libgbm.so.1','libEGL.so.1','libGLESv2.so.2']}
    def fn(lib,name,restype,args):
        f=libs[lib].__getattr__(name);f.restype=restype;f.argtypes=args;return f
    create=fn('libgbm.so.1','gbm_create_device',p,[i])
    destroy=fn('libgbm.so.1','gbm_device_destroy',None,[p])
    getproc=fn('libEGL.so.1','eglGetProcAddress',p,[C.c_char_p])
    getdisplay=C.CFUNCTYPE(p,u,p,C.POINTER(i))(getproc(b'eglGetPlatformDisplayEXT'))
    init=fn('libEGL.so.1','eglInitialize',u,[p,C.POINTER(i),C.POINTER(i)])
    query=fn('libEGL.so.1','eglQueryString',C.c_char_p,[p,i])
    choose=fn('libEGL.so.1','eglChooseConfig',u,[p,C.POINTER(i),C.POINTER(p),i,C.POINTER(i)])
    bind=fn('libEGL.so.1','eglBindAPI',u,[u])
    context=fn('libEGL.so.1','eglCreateContext',p,[p,p,p,C.POINTER(i)])
    surface=fn('libEGL.so.1','eglCreatePbufferSurface',p,[p,p,C.POINTER(i)])
    current=fn('libEGL.so.1','eglMakeCurrent',u,[p,p,p,p])
    destroy_surface=fn('libEGL.so.1','eglDestroySurface',u,[p,p])
    destroy_context=fn('libEGL.so.1','eglDestroyContext',u,[p,p])
    terminate=fn('libEGL.so.1','eglTerminate',u,[p])
    error=fn('libEGL.so.1','eglGetError',i,[])
    def checked(value,op):
        if not value:raise RuntimeError(op+': EGL error '+hex(error()))
        return value
    fd=os.open('/dev/dri/renderD128',os.O_RDWR|os.O_CLOEXEC)
    gbm=display=ctx=sfc=None
    result={'presented':False,'drm_master_requested':False,'render_node':'/dev/dri/renderD128'}
    try:
        gbm=checked(create(fd),'gbm_create_device')
        display=checked(getdisplay(0x31D7,gbm,None),'eglGetPlatformDisplayEXT(GBM)')
        major=i();minor=i();checked(init(display,C.byref(major),C.byref(minor)),'eglInitialize')
        for label,code in [('EGL_VENDOR',0x3053),('EGL_VERSION',0x3054),('EGL_CLIENT_APIS',0x308D),('EGL_EXTENSIONS',0x3055)]:
            result[label]=(query(display,code) or b'').decode()
        cfg=p();n=i();attrs=(i*15)(0x3033,1,0x3040,4,0x3024,8,0x3023,8,0x3022,8,0x3021,0,0x3025,0,0x3038)
        checked(choose(display,attrs,C.byref(cfg),1,C.byref(n)) and n.value,'eglChooseConfig')
        checked(bind(0x30A0),'eglBindAPI GLES')
        ctx=checked(context(display,cfg,None,(i*3)(0x3098,2,0x3038)),'eglCreateContext GLES2')
        sfc=checked(surface(display,cfg,(i*5)(0x3057,1,0x3056,1,0x3038)),'eglCreatePbufferSurface')
        checked(current(display,sfc,sfc,ctx),'eglMakeCurrent')
        glstring=fn('libGLESv2.so.2','glGetString',C.c_char_p,[u])
        for label,code in [('GL_VENDOR',0x1F00),('GL_RENDERER',0x1F01),('GL_VERSION',0x1F02),('GL_EXTENSIONS',0x1F03)]:result[label]=(glstring(code) or b'').decode()
        clearcolor=fn('libGLESv2.so.2','glClearColor',None,[C.c_float]*4)
        clear=fn('libGLESv2.so.2','glClear',None,[u]);read=fn('libGLESv2.so.2','glReadPixels',None,[i,i,i,i,u,u,p])
        clearcolor(.25,.5,.75,1);clear(0x4000);pixel=(C.c_ubyte*4)();read(0,0,1,1,0x1908,0x1401,pixel)
        result['pixel_rgba']=list(pixel);result['GL_ERROR']=hex(fn('libGLESv2.so.2','glGetError',u,[])())
        name=result['GL_RENDERER'].lower();result['hardware_mali']=('mali' in name and not any(x in name for x in ['llvmpipe','softpipe','swrast','software']))
        result['mapped_gpu_libraries']=sorted({line.split()[-1] for line in open('/proc/self/maps') if any(x in line for x in ['libmali','libEGL','libGLES','libgbm'])})
    finally:
        if display:
            current(display,None,None,None)
            if sfc:destroy_surface(display,sfc)
            if ctx:destroy_context(display,ctx)
            terminate(display)
        if gbm:destroy(gbm)
        os.close(fd)
    print(json.dumps(result,indent=2))
    return 0 if result.get('hardware_mali') and result.get('GL_ERROR')=='0x0' else 2

if __name__=='__main__':raise SystemExit(main())
