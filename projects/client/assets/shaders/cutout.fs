#version 330 core
out vec4 FragColor;
  
in vec3 col;
in vec4 st;

uniform sampler2D tex;
uniform sampler2D lightmap;

void main()
{
	vec4 tex = texture(tex, st.xy);
	if(abs(tex.x - 0.62353) + abs(tex.y - 0.356863) + abs(tex.z - 0.3254902) < 0.01) discard;
    FragColor = vec4(col, 1.0) * tex * (texture(lightmap, st.zw) * 4f);
}
