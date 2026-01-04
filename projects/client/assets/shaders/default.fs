#version 330 core
out vec4 FragColor;
  
in vec4 col;
in vec4 st;

uniform sampler2D tex;
uniform sampler2D lightmap;

void main()
{
    FragColor = col * texture(tex, st.xy) * texture(lightmap, st.zw);
	//FragColor = texture(lightmap, st.zw);
}