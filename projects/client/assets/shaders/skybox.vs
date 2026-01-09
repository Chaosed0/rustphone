#version 330 core
layout (location = 0) in vec3 in_pos;
layout (location = 1) in vec3 in_col;
layout (location = 2) in vec4 in_st;

out vec3 col;
out vec3 skyCoords;

uniform mat4 mvp;
uniform vec3 eyePos;

void main()
{
	vec4 pos = mvp * vec4(in_pos.y, in_pos.z, in_pos.x, 1.0);
    gl_Position = pos.xyww;
    col = in_col;
	skyCoords = vec3(in_pos.y, in_pos.z, in_pos.x) - eyePos;
}