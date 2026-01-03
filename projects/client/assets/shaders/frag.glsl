#version 330 core
out vec4 FragColor;
  
in vec4 col;
in vec4 st;

uniform sampler2D tex;

void main()
{
    FragColor = col * texture(tex, st.xy);
	//FragColor = vec4(1.0f, 0.5f, 0.2f, 1.0f);
}