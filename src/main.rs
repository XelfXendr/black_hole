use nalgebra::{Rotation3, Vector3, Unit};
use image::{RgbImage, Rgb};
use std::time::Instant;
use std::thread;
use std::collections::VecDeque;

const C: f64 = 299792458.0;
const M: f64 = 8.3e36;
const G: f64 = 6.6743e-11;
const RS: f64 = 2f64 / 3f64 * G * M / C / C; //it should be 3 times larger...
const SCALE: f64 = 20f64 / RS;

/*
TODO:
    Perspective camera
    Improve multithreading (don't spawn gazillion threads at the same time)
    Forget scale
    Ask for user input (resolution, sampling, position, FOV)
    Use better colors (and textures?) for ring and skybox
*/

fn main() {
    let pos_blackhole: Vector3<f64> = Vector3::new(100f64,100f64,100f64) / SCALE;
    let mut img = RgbImage::new(200, 200);
    let mut q: VecDeque<thread::JoinHandle<(u32, u32, Rgb<u8>)>> = VecDeque::new();
    
    let now = Instant::now();
    for j in 0..200
    {
        for i in 0..200
        {
            let i_clone = i;
            let j_clone = j;
            let pos_blackhole_clone = pos_blackhole;
            let t = thread::spawn(move || {
                return (i_clone, j_clone, send_ray(pos_blackhole_clone, Vector3::new(i_clone as f64, j_clone as f64 - 30f64, -50f64)/SCALE, Vector3::new(0f64, 0.2f64, 1f64))); 
            });

            q.push_back(t);
        }
    }

    for r in q
    {
        let (i, j, rgb) = r.join().unwrap();
        img.put_pixel(i, j, rgb);
    }
    println!("{:?}", now.elapsed());
    
    img.save("image.png").unwrap();
}

fn send_ray(pos_blackhole: Vector3<f64>, mut pos_photon: Vector3<f64>, mut dir_photon: Vector3<f64>) -> Rgb<u8>
{
    dir_photon = *Unit::new_normalize(dir_photon) * C;
    let mut delta_time: f64;
    loop
    {
        let dist = (pos_blackhole - pos_photon).magnitude();
        if dist < RS 
        {
            return Rgb([0,0,0]);
        }
        if dist > RS*10f64
        {
            return Rgb([0,0,50]);
        }

        if dist < 2f64*RS
        {
            delta_time = 0.001f64;
        }
        else
        {
            delta_time = dist / RS - 2f64;
            delta_time = 0.999 / 4096.0 * delta_time * delta_time * delta_time * delta_time + 0.001;
        }

        let dist_vec = pos_blackhole - pos_photon;
        let dist: f64 = dist_vec.magnitude();
        let force = G * M / dist / dist;
        let delta_theta = (dist_vec.angle(&dir_photon)).sin() * force / C * delta_time;
        let u = dir_photon.cross(&dist_vec);
        if u.magnitude() != 0f64
        {
            let rotation_matrix = Rotation3::from_axis_angle(&Unit::new_normalize(u), delta_theta);
            dir_photon = rotation_matrix * dir_photon;
        }
        let new_pos = pos_photon + dir_photon * delta_time;

        if (((new_pos.y - pos_blackhole.y) < 0f64) ^ ((pos_photon.y - pos_blackhole.y) < 0f64)) && dist > 3f64*RS && dist < 6f64*RS
        {
            return Rgb([255, 128, 0]);
        }
        pos_photon = new_pos;
    }
}