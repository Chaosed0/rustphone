#version 330 core
out vec4 FragColor;
  
in vec4 col;
in vec2 texCoord;
in vec3 light;

uniform sampler2D tex;

void main()
{
    //FragColor = col * texture(tex, texCoord) * vec4(light, 1.0);
    FragColor = vec4(light, 1.0);
}
