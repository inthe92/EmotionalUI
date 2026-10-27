in vec2 a_pos;
uniform vec4 u_rect;
uniform vec2 u_res;
out vec2 v_uv;
void main(){v_uv=a_pos;vec2 p=u_rect.xy+a_pos*u_rect.zw;
gl_Position=vec4(p.x/u_res.x*2.0-1.0,1.0-p.y/u_res.y*2.0,0.0,1.0);}
