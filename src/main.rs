use nalgebra::{Rotation3, Vector3, Unit};
use image::{RgbImage, Rgb};
use std::time::Instant;
use std::cmp::{min, max};
use num_cpus;
use std::thread::{spawn, JoinHandle};
use std::collections::VecDeque;
use std::io::{stdin, stdout, Write};
use std::str::FromStr;
use std::f64::consts::PI;

const C: f64 = 299792458.0;
const M: f64 = 8.3e36;
const G: f64 = 6.6743e-11;
const RS: f64 = 2f64 / 3f64 * G * M / C / C; //it's 3 time smaller than it should be

/*
    Position of the black hole is [0;0;0]
*/

/*
TODO:
    Use better colors (and textures?) for ring and skybox
*/

fn main() {
    //Getting input
    let back: f64 = get_input("Position of the camera on x in multiple of Rs (default: 10): ", "Invalid input. Try again.", 10f64) * RS;
    let up: f64 = get_input("Position of the camera on y in multiple of Rs (default: 1): ", "Invalid input. Try again.", 1f64) * RS;
    let fov_horizontal: f64 = get_input("FOV in degrees (default: 120): ", "Invalid input. Try again.", 120f64) * PI / 180f64;
    let samples: usize = get_input("Samples per pixel width (default: 1): ", "Invalid input. Try again.", 1usize);
    let mut res = String::new();
    let (img_width, img_height): (u32, u32) = loop {
        print!("Output resolution (default: 512x288): ");
        stdout().flush().unwrap();
        match stdin().read_line(&mut res)
        {
            Ok(_) => {
                if res.trim().len() == 0
                {
                    break (512, 288);
                }
                let parts: Vec<&str> = res.trim().split('x').collect();
                if parts.len() != 2 {
                    println!("Invalid input. Try again;");
                    continue;
                }
                break ( match parts[0].parse(){
                    Ok(n) => n,
                    Err(_) => {
                        println!("Invalid first number. Try again.");
                        continue;
                    }
                }, match parts[1].parse(){
                    Ok(n) => n,
                    Err(_) => {
                        println!("Invalid second number. Try again.");
                        continue;
                    }
                })
            },
            Err(_) => {
                println!("Error reading input. Try again.");
                continue;
            }
        }
    };

    //variables
    let degrees_per_pixel = fov_horizontal / img_width as f64;
    let fov_vertical = degrees_per_pixel * img_height as f64;

    let camera_position: Vector3<f64> = Vector3::new(0f64, up, -back);
    let camera_vertical_angle: f64 = (back / camera_position.magnitude()).acos() * up.signum();

    let mut threads_available = num_cpus::get();
    let mut img = RgbImage::new(img_width, img_height);

    let mut pixel_thread_queue: VecDeque<JoinHandle<(u32, u32, usize, Rgb<u8>)>> = VecDeque::new();

    //drawing pixels
    for j in 0..img_height
    {
        for i in 0..img_width
        {
            if threads_available == 0
            {
                threads_available += join_pixel_thread(pixel_thread_queue.pop_front().unwrap(), &mut img);
            }

            let threads_to_use = min(threads_available, samples * samples);
            threads_available -= threads_to_use;

            let pixel_horizontal_angle = degrees_per_pixel * i as f64 - fov_horizontal / 2f64;
            let pixel_vertical_angle = degrees_per_pixel * j as f64 - fov_vertical / 2f64 + camera_vertical_angle;
            
            let (i_clone, j_clone, samples_clone, camera_position_clone, degrees_per_pixel_clone) = (i, j, samples, camera_position, degrees_per_pixel);

            //pixel thread
            pixel_thread_queue.push_back( spawn( move || {
                let pixel = get_pixel(camera_position_clone, pixel_vertical_angle, pixel_horizontal_angle, degrees_per_pixel_clone, samples_clone, threads_to_use);
                return (i_clone, j_clone, threads_to_use, pixel);
            }));
        }
        println!("{}/{}", j+1, img_height);
    }
    for h in pixel_thread_queue
    {
        join_pixel_thread(h, &mut img);
    }

    img.save("image.png").unwrap();
}

fn get_pixel(cam_pos: Vector3<f64>, vertical_angle: f64, horizontal_angle: f64, degrees_per_pixel: f64, samples: usize, threads_to_use: usize) -> Rgb<u8>
{
    let mut ray_thread_queue: VecDeque<JoinHandle<Rgb<u8>>> = VecDeque::new();
    let mut colors: Vec<Rgb<u8>> = Vec::new();
    let mut ray_threads_available = threads_to_use;
    for p_j in 0..samples
    {
        for p_i in 0..samples
        {
            if ray_threads_available == 0
            {
                colors.push(ray_thread_queue.pop_front().unwrap().join().unwrap());
                ray_threads_available += 1;
            }

            let ray_horizontal_angle = degrees_per_pixel * (p_i as f64 + 0.5) / samples as f64 - degrees_per_pixel / 2f64 + horizontal_angle;
            let ray_vertical_angle = degrees_per_pixel * (p_j as f64 + 0.5) / samples as f64 - degrees_per_pixel / 2f64 + vertical_angle;

            let ray_dir: Vector3<f64> = Vector3::new(ray_vertical_angle.cos() * ray_horizontal_angle.sin(), -ray_vertical_angle.sin(), ray_vertical_angle.cos()*ray_horizontal_angle.cos());
            let camera_position_clone = cam_pos;

            //ray thread
            ray_thread_queue.push_back( spawn( move || {
                return send_ray(camera_position_clone, ray_dir);
            }));
            ray_threads_available -= 1;
        }
    }
    for t in ray_thread_queue
    {
        colors.push(t.join().unwrap());
    }

    let (mut r, mut g, mut b): (u32, u32, u32) = (0,0,0);
    let color_count = colors.len() as u32;
    for c in colors
    {
        r += c[0] as u32;
        g += c[1] as u32;
        b += c[2] as u32;
    }
    return Rgb([(r / color_count) as u8, (g / color_count) as u8, (b / color_count) as u8]);
}

fn send_ray(mut pos_photon: Vector3<f64>, mut dir_photon: Vector3<f64>) -> Rgb<u8>
{
    dir_photon = *Unit::new_normalize(dir_photon) * C;
    let mut delta_time: f64;
    loop
    {
        let dist = pos_photon.magnitude();
        if dist < RS 
        {
            return Rgb([0,0,0]);
        }
        if dist > RS*20f64
        {
            return Rgb([0,0,10]);
        }

        if dist < 2f64*RS
        {
            delta_time = 0.001f64;
        }
        else
        {
            delta_time = dist / RS - 2f64;
            delta_time = 0.999 / 4096.0 * delta_time * delta_time * delta_time * delta_time + 0.001;
            if delta_time > 1f64
            {
                delta_time = 1f64;
            }
        }

        let force = G * M / dist / dist;
        let delta_theta = (pos_photon.angle(&dir_photon)).sin() * force / C * delta_time;
        let u = dir_photon.cross(&-pos_photon);
        if u.magnitude() != 0f64
        {
            let rotation_matrix = Rotation3::from_axis_angle(&Unit::new_normalize(u), delta_theta);
            dir_photon = rotation_matrix * dir_photon;
        }
        let new_pos = pos_photon + dir_photon * delta_time;

        if ((new_pos.y < 0f64) ^ (pos_photon.y < 0f64)) && dist > 3f64*RS && dist < 6f64*RS
        {
            return Rgb([255, 200, 0]);
        }
        pos_photon = new_pos;
    }
}

fn join_pixel_thread(handle: JoinHandle<(u32, u32, usize, Rgb<u8>)>, img: &mut RgbImage) -> usize
{
    let (x, y, threads, rgb) = handle.join().unwrap();
    img.put_pixel(x, y, rgb);
    return threads;
}

fn get_input<T: FromStr>(message: &str, error_message: &str, default: T) -> T
{
    let mut input = String::new();
    return loop {
        print!("{}", message);
        stdout().flush().unwrap();
        match stdin().read_line(&mut input)
        {
            Ok(_) => {
                if input.trim().len() == 0
                {
                    break default;
                }
                match input.trim().parse::<T>() {   
                    Ok(n) => break n,
                    Err(_) => {
                        println!("{}", error_message);
                        continue;
                    }
                }
            },
            Err(_) => {
                println!("{}", error_message);
                continue;
            }
        }
    };
}