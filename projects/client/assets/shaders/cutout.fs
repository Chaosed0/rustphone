#version 330 core
out vec4 FragColor;
  
in vec4 col;
in vec4 st;

uniform sampler2D tex;

void main()
{
	vec4 tex = texture(tex, st.xy);
	if(abs(tex.x - 0.62353) + abs(tex.y - 0.356863) + abs(tex.z - 0.3254902) < 0.01) discard;
    FragColor = col * tex;
}