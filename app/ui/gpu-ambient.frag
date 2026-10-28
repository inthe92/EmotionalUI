in vec2 v_uv;out vec4 color;
uniform sampler2D u_day;uniform sampler2D u_night;
uniform float u_mix;uniform vec3 u_tint;uniform float u_pan;
void main(){
    // Preserve the existing room plane's 664x498 overscan and 6px travel.
    vec2 uv=(v_uv-0.5)*vec2(640.0/664.0,480.0/498.0)+0.5;
    uv.x-=u_pan/664.0;
    vec3 c=mix(texture(u_day,uv).rgb,texture(u_night,uv).rgb,u_mix);
    c=mix(c,c*u_tint,0.20);
    float shade=smoothstep(0.35,1.0,v_uv.y)*0.25;
    color=vec4(c*(1.0-shade),1.0);
}
