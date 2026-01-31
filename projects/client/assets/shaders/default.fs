#version 330 core
out vec4 FragColor;
  
in vec3 col;
in vec4 st;

uniform sampler2D tex;
uniform sampler2D lightmap;

void main()
{
    FragColor = vec4(col, 1.0) * texture(tex, st.xy) * (texture(lightmap, st.zw) * 4f);
	//FragColor = texture(lightmap, st.zw);
}
