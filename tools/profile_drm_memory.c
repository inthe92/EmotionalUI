/* SPDX-License-Identifier: GPL-3.0-only
 * Compare alpha blending in cached RAM with a newly allocated DRM dumb mmap.
 * Does NOT acquire DRM master, add framebuffer, set mode, page flip, or touch
 * any existing framebuffer. All writes address this process's new dumb buffer.
 * Explicit Linux device argument required; this is diagnostic, not a benchmark
 * of Slint or presentation. Example: profile_drm_memory /dev/dri/card0
 */
#define _POSIX_C_SOURCE 200809L
#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <time.h>
#include <unistd.h>
struct create_dumb { uint32_t height,width,bpp,flags,handle,pitch; uint64_t size; };
struct map_dumb { uint32_t handle,pad; uint64_t offset; };
struct destroy_dumb { uint32_t handle; };
#define CREATE_DUMB _IOWR('d',0xB2,struct create_dumb)
#define MAP_DUMB _IOWR('d',0xB3,struct map_dumb)
#define DESTROY_DUMB _IOWR('d',0xB4,struct destroy_dumb)
static void deadline(int sig) { (void)sig; static const char msg[]="Diagnostic timeout; process-owned DRM allocation released on exit.\n"; write(2,msg,sizeof(msg)-1); _exit(124); }
static uint64_t checksum(const uint32_t *p,size_t n) { uint64_t h=1469598103934665603ULL; for(size_t i=0;i<n;i++) { h^=p[i]; h*=1099511628211ULL; } return h; }
static double now_ms(void) { struct timespec t; clock_gettime(CLOCK_MONOTONIC,&t); return t.tv_sec*1000. + t.tv_nsec/1e6; }
static void blend(uint32_t *p, size_t stride, unsigned passes) {
    for(unsigned pass=0;pass<passes;pass++) for(size_t y=0;y<480;y++) for(size_t x=0;x<640;x++) {
        uint32_t v=p[y*stride+x];
        uint32_t r=(((v>>16)&255)*159/255)+30;
        uint32_t g=(((v>>8)&255)*159/255)+24;
        uint32_t b=((v&255)*159/255)+20;
        p[y*stride+x]=0xff000000|(r<<16)|(g<<8)|b;
    }
}
int main(int argc,char **argv) {
    if(argc!=2 || strncmp(argv[1],"/dev/dri/card",13)!=0) { fputs("Explicit /dev/dri/cardN required; creates only an isolated diagnostic buffer.\n",stderr); return 2; }
    int fd=open(argv[1],O_RDWR|O_CLOEXEC); if(fd<0) { perror("open DRM"); return 1; }
    struct create_dumb d={.height=480,.width=640,.bpp=32};
    struct map_dumb m={0}; struct destroy_dumb destroy={0};
    uint32_t *mapped=MAP_FAILED,*ram=NULL; int status=1;
    if(ioctl(fd,CREATE_DUMB,&d)<0) { perror("CREATE_DUMB"); goto done; }
    destroy.handle=d.handle; m.handle=d.handle;
    if(d.pitch<640*4 || d.pitch%4 || d.size<(uint64_t)d.pitch*480 || d.size>16*1024*1024) { fputs("Unexpected allocation bounds\n",stderr); goto done; }
    if(ioctl(fd,MAP_DUMB,&m)<0) { perror("MAP_DUMB"); goto done; }
    mapped=mmap(NULL,(size_t)d.size,PROT_READ|PROT_WRITE,MAP_SHARED,fd,(off_t)m.offset);
    if(mapped==MAP_FAILED) { perror("mmap"); goto done; }
    ram=calloc(1,(size_t)d.size); if(!ram) { perror("calloc"); goto done; }
    memset(mapped,0,(size_t)d.size);
    signal(SIGALRM,deadline); alarm(5);
    printf("{\"width\":640,\"height\":480,\"pitch\":%u,\"bytes\":%llu,\"passes\":5,\"display_modified\":false,\"samples\":[",d.pitch,(unsigned long long)d.size);
    for(unsigned n=0;n<3;n++) {
        double start=now_ms(); blend(ram,d.pitch/4,5); double ram_ms=now_ms()-start;
        start=now_ms(); blend(mapped,d.pitch/4,5); double mmap_ms=now_ms()-start;
        start=now_ms(); blend(ram,d.pitch/4,5); memcpy(mapped,ram,(size_t)d.size); double shadow_ms=now_ms()-start;
        uint32_t actual=mapped[0], expected=ram[0];
        printf("%s{\"ram_ms\":%.4f,\"mmap_ms\":%.4f,\"ram_plus_copy_ms\":%.4f,\"copy_valid\":%s}",n?",":"",ram_ms,mmap_ms,shadow_ms,actual==expected?"true":"false");
        fflush(stdout);
    }
    uint64_t a=checksum(ram,(size_t)d.size/4),b=checksum(mapped,(size_t)d.size/4);
    printf("],\"ram_checksum\":\"%016llx\",\"mapped_checksum\":\"%016llx\",\"equal_checksum\":%s}\n",(unsigned long long)a,(unsigned long long)b,a==b?"true":"false");
    alarm(0); status=a==b?0:1;
 done:
    free(ram); if(mapped!=MAP_FAILED) munmap(mapped,(size_t)d.size);
    if(destroy.handle && ioctl(fd,DESTROY_DUMB,&destroy)<0) { perror("DESTROY_DUMB"); status=1; }
    close(fd); return status;
}
