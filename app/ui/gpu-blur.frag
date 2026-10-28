in vec2 v_uv;out vec4 color;uniform sampler2D u_tex;uniform vec2 u_step;
// Separable Gaussian: five bilinear samples per pass, at half/quarter resolution.
void main(){vec2 uv=vec2(v_uv.x,1.0-v_uv.y);
color=texture(u_tex,uv)*0.227027;
color+=(texture(u_tex,uv+u_step*1.384615)+texture(u_tex,uv-u_step*1.384615))*0.316216;
color+=(texture(u_tex,uv+u_step*3.230769)+texture(u_tex,uv-u_step*3.230769))*0.070270;}
