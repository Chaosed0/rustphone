#version 330 core
out vec4 FragColor;
  
in vec3 col;
in vec3 skyCoords;

uniform samplerCube skybox;

void main()
{
    FragColor = vec4(col, 1.0) * texture(skybox, skyCoords);
}
