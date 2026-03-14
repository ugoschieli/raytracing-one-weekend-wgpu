use glam::Vec3;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SphereUniform {
    pub mat: MaterialUniform,
    pub center: Vec3,
    pub radius: f32,
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

#[derive(Debug, Clone, Copy)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
    pub mat: Material,
}

impl Sphere {
    pub fn new(center: Vec3, radius: f32, mat: Material) -> Self {
        Self {
            center,
            radius,
            mat,
        }
    }

    pub fn to_uniform(&self) -> SphereUniform {
        SphereUniform {
            mat: self.mat.to_uniform(),
            center: self.center,
            radius: self.radius,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Material {
    Lambertian(Vec3),
    Metal(Vec3, f32),
    Dielectric(f32),
}

pub const MAT_LAMBERTIAN: u32 = 0;
pub const MAT_METAL: u32 = 1;
pub const MAT_DIELECTRIC: u32 = 2;

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

pub struct World {
    pub spheres: Vec<Sphere>,
}

impl World {
    pub fn new(spheres: &[Sphere]) -> Self {
        Self {
            spheres: spheres.to_vec(),
        }
    }

    pub fn to_uniform(&self) -> Vec<SphereUniform> {
        self.spheres
            .iter()
            .map(|sphere| sphere.to_uniform())
            .collect()
    }
}
