use bytemuck::{Pod, Zeroable};
use glam::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct Cube {
    pub position: Vec3,
    pub size: f32,
    pub material: Material,
}

#[derive(Debug, Clone, Copy)]
pub enum Material {
    Lambertian(Vec3),
    Metal(Vec3, f32),
    Dielectric(f32),
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CubeUniform {
    pub mat: MaterialUniform,
    pub center: Vec3,
    pub size: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialUniform {
    pub mat_type: u32,
    pub roughness: f32,
    pub refraction_index: f32,
    _pad: u32,
    pub albedo: Vec3,
    _pad1: u32,
}

pub const MAT_LAMBERTIAN: u32 = 0;
pub const MAT_METAL: u32 = 1;
pub const MAT_DIELECTRIC: u32 = 2;

pub struct World {
    pub cubes: Vec<Cube>,
}

impl Cube {
    pub fn new(position: Vec3, size: f32, material: Material) -> Self {
        Self {
            position,
            size,
            material,
        }
    }

    pub fn to_uniform(&self) -> CubeUniform {
        CubeUniform {
            mat: self.material.to_uniform(),
            center: self.position,
            size: self.size,
        }
    }
}

impl World {
    pub fn new(cubes: &[Cube]) -> Self {
        Self {
            cubes: cubes.to_vec(),
        }
    }

    pub fn to_uniform(&self) -> Vec<CubeUniform> {
        self.cubes.iter().map(|cube| cube.to_uniform()).collect()
    }
}

impl Material {
    pub fn to_uniform(&self) -> MaterialUniform {
        match self {
            Material::Lambertian(albedo) => MaterialUniform {
                mat_type: MAT_LAMBERTIAN,
                roughness: 0.0,
                refraction_index: 0.0,
                _pad: 0,
                albedo: *albedo,
                _pad1: 0,
            },
            Material::Metal(albedo, roughness) => MaterialUniform {
                mat_type: MAT_METAL,
                roughness: *roughness,
                refraction_index: 0.0,
                _pad: 0,
                albedo: *albedo,
                _pad1: 0,
            },
            Material::Dielectric(refraction_index) => MaterialUniform {
                mat_type: MAT_DIELECTRIC,
                roughness: 0.0,
                refraction_index: *refraction_index,
                _pad: 0,
                albedo: Vec3::ZERO,
                _pad1: 0,
            },
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

impl Vertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub const CUBE_VERTICES: &[Vertex] = &[
    // Front face (Z = 1)
    Vertex {
        position: [-1.0, -1.0, 1.0],
        normal: [0.0, 0.0, 1.0],
    },
    Vertex {
        position: [1.0, -1.0, 1.0],
        normal: [0.0, 0.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0, 1.0],
        normal: [0.0, 0.0, 1.0],
    },
    Vertex {
        position: [-1.0, 1.0, 1.0],
        normal: [0.0, 0.0, 1.0],
    },
    // Back face (Z = -1)
    Vertex {
        position: [-1.0, -1.0, -1.0],
        normal: [0.0, 0.0, -1.0],
    },
    Vertex {
        position: [-1.0, 1.0, -1.0],
        normal: [0.0, 0.0, -1.0],
    },
    Vertex {
        position: [1.0, 1.0, -1.0],
        normal: [0.0, 0.0, -1.0],
    },
    Vertex {
        position: [1.0, -1.0, -1.0],
        normal: [0.0, 0.0, -1.0],
    },
    // Top face (Y = 1)
    Vertex {
        position: [-1.0, 1.0, -1.0],
        normal: [0.0, 1.0, 0.0],
    },
    Vertex {
        position: [-1.0, 1.0, 1.0],
        normal: [0.0, 1.0, 0.0],
    },
    Vertex {
        position: [1.0, 1.0, 1.0],
        normal: [0.0, 1.0, 0.0],
    },
    Vertex {
        position: [1.0, 1.0, -1.0],
        normal: [0.0, 1.0, 0.0],
    },
    // Bottom face (Y = -1)
    Vertex {
        position: [-1.0, -1.0, -1.0],
        normal: [0.0, -1.0, 0.0],
    },
    Vertex {
        position: [1.0, -1.0, -1.0],
        normal: [0.0, -1.0, 0.0],
    },
    Vertex {
        position: [1.0, -1.0, 1.0],
        normal: [0.0, -1.0, 0.0],
    },
    Vertex {
        position: [-1.0, -1.0, 1.0],
        normal: [0.0, -1.0, 0.0],
    },
    // Right face (X = 1)
    Vertex {
        position: [1.0, -1.0, -1.0],
        normal: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [1.0, 1.0, -1.0],
        normal: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [1.0, 1.0, 1.0],
        normal: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [1.0, -1.0, 1.0],
        normal: [1.0, 0.0, 0.0],
    },
    // Left face (X = -1)
    Vertex {
        position: [-1.0, -1.0, -1.0],
        normal: [-1.0, 0.0, 0.0],
    },
    Vertex {
        position: [-1.0, -1.0, 1.0],
        normal: [-1.0, 0.0, 0.0],
    },
    Vertex {
        position: [-1.0, 1.0, 1.0],
        normal: [-1.0, 0.0, 0.0],
    },
    Vertex {
        position: [-1.0, 1.0, -1.0],
        normal: [-1.0, 0.0, 0.0],
    },
];

pub const CUBE_INDICES: &[u16] = &[
    0, 1, 2, 0, 2, 3, // front
    4, 5, 6, 4, 6, 7, // back
    8, 9, 10, 8, 10, 11, // top
    12, 13, 14, 12, 14, 15, // bottom
    16, 17, 18, 16, 18, 19, // right
    20, 21, 22, 20, 22, 23, // left
];
