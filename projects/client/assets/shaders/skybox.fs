#version 330 core
out vec4 FragColor;
  
in vec4 col;
in vec3 skyCoords;

uniform samplerCube skybox;

void main()
{
    FragColor = col * texture(skybox, skyCoords);
}