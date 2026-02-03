#version 430 core

struct LightgridData {
	vec3 dist;
	int size[3];
	vec3 mins;
};

layout (location = 0) in vec3 vertexPosition;
layout (location = 1) in vec4 vertexColor;
layout (location = 2) in vec2 vertexTexCoord;

layout (std430, binding = 3) buffer LightgridBuffer {
	vec4 samples[];
} lightgridBuffer;

out vec4 col;
out vec3 light;
out vec2 texCoord;

uniform mat4 mvp;
uniform LightgridData lgData;

int lg_index(int x, int y, int z)
{
	return x + y * lgData.size[0] + z * lgData.size[0] * lgData.size[1];
}

vec4 lg_avg(vec4 sample1, vec4 sample2, float val)
{
	bool s1occluded = sample1.w < 0.5;
	bool s2occluded = sample2.w < 0.5;

	if (!s1occluded && !s2occluded) {
		return vec4(mix(sample1.rgb, sample2.rgb, val), 1.0);
	} else if (s1occluded && s2occluded) {
		return vec4(0.0, 0.0, 0.0, 0.0);
	} else if (!s1occluded) {
		return sample1;
	} else {
		return sample2;
	}
}

void main()
{
	vec3 vert = vertexPosition;

	vec3 lgGridpos = (vert - lgData.mins) / lgData.dist;
	int lgGridX = int(clamp(lgGridpos.x, 0.0, float(lgData.size[0])));
	int lgGridY = int(clamp(lgGridpos.y, 0.0, float(lgData.size[1])));
	int lgGridZ = int(clamp(lgGridpos.z, 0.0, float(lgData.size[2])));

	vec4 samplex0y0z0 = lightgridBuffer.samples[lg_index(lgGridX + 0, lgGridY + 0, lgGridZ + 0)];
	vec4 samplex1y0z0 = lightgridBuffer.samples[lg_index(lgGridX + 1, lgGridY + 0, lgGridZ + 0)];
	vec4 samplex0y1z0 = lightgridBuffer.samples[lg_index(lgGridX + 0, lgGridY + 1, lgGridZ + 0)];
	vec4 samplex1y1z0 = lightgridBuffer.samples[lg_index(lgGridX + 1, lgGridY + 1, lgGridZ + 0)];
	vec4 samplex0y0z1 = lightgridBuffer.samples[lg_index(lgGridX + 0, lgGridY + 0, lgGridZ + 1)];
	vec4 samplex1y0z1 = lightgridBuffer.samples[lg_index(lgGridX + 1, lgGridY + 0, lgGridZ + 1)];
	vec4 samplex0y1z1 = lightgridBuffer.samples[lg_index(lgGridX + 0, lgGridY + 1, lgGridZ + 1)];
	vec4 samplex1y1z1 = lightgridBuffer.samples[lg_index(lgGridX + 1, lgGridY + 1, lgGridZ + 1)];

	vec4 samplex0y0 = lg_avg(samplex0y0z0, samplex0y0z1, fract(lgGridpos.z));
	vec4 samplex0y1 = lg_avg(samplex0y1z0, samplex0y1z1, fract(lgGridpos.z));
	vec4 samplex1y0 = lg_avg(samplex1y0z0, samplex1y0z1, fract(lgGridpos.z));
	vec4 samplex1y1 = lg_avg(samplex1y1z0, samplex1y1z1, fract(lgGridpos.z));
	
	vec4 samplex0 = lg_avg(samplex0y0, samplex0y1, fract(lgGridpos.y));
	vec4 samplex1 = lg_avg(samplex1y0, samplex1y1, fract(lgGridpos.y));

	vec4 sample_final = lg_avg(samplex0, samplex1, fract(lgGridpos.x));

    gl_Position = mvp * vec4(vert, 1.0);
    col = vertexColor;
	texCoord = vertexTexCoord;
	light = sample_final.w > 0.5 ? sample_final.xyz : vec3(0,0,0);
}