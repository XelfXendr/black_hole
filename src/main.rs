use nalgebra::{Rotation3, Vector3, Unit};
use image::{RgbImage, Pixel, Rgb};
use std::time::{Instant};
use std::cmp::{min, max};
use num_cpus;
use std::thread::{spawn};
use std::io::{stdin, stdout, Write};
use std::str::FromStr;
use std::f64::consts::PI;
use lazy_static::lazy_static;
use chrono::{Local};
use std::sync::mpsc;

const C: f64 = 299792458.0; //speed of light [m/s]
const M: f64 = 8.3e36; //mass of the black hole [kg]
const G: f64 = 6.6743e-11; //gravitational constant
const RS: f64 = 2f64 / 3f64 * G * M / C / C; //it's 3 times smaller than it should be 🤔
//Position of the black hole is [0;0;0]

/*
TODO:
    doppler effect?
    user input textures
    user input accretion disc size
    comments

BUGs:
    wrong input makes infinite loop?
*/

lazy_static! {
    //textures
    static ref ACCRETION_IMG: RgbImage = image::open("textures/accretion_disc.png").unwrap().to_rgb();
    static ref SKYBOX_FRONT: RgbImage = image::open("textures/starbox_dimmer/skyboxfront.png").unwrap().to_rgb();
    static ref SKYBOX_RIGHT: RgbImage = image::open("textures/starbox_dimmer/skyboxright.png").unwrap().to_rgb();
    static ref SKYBOX_BACK: RgbImage = image::open("textures/starbox_dimmer/skyboxback.png").unwrap().to_rgb();
    static ref SKYBOX_LEFT: RgbImage = image::open("textures/starbox_dimmer/skyboxleft.png").unwrap().to_rgb();
    static ref SKYBOX_TOP: RgbImage = image::open("textures/starbox_dimmer/skyboxtop.png").unwrap().to_rgb();
    static ref SKYBOX_BOTTOM: RgbImage = image::open("textures/starbox_dimmer/skyboxbottom.png").unwrap().to_rgb();
    static ref R_ISCO_PX: u32 = 150u32;

    //user input variables
    static ref BACK: f64 = get_input("Position of the camera on x in multiple of Rs (default: 15): ", "Invalid input. Try again.", 15f64) * RS;
    static ref UP: f64 = get_input("Position of the camera on y in multiple of Rs (default: 1): ", "Invalid input. Try again.", 1f64) * RS;
    static ref CAMERA_POSITION: Vector3<f64> = Vector3::new(0f64, *UP, -*BACK);
    static ref CAMERA_VERTICAL_ANGLE: f64 = (*BACK / CAMERA_POSITION.magnitude()).acos() * UP.signum();
    
    static ref FOV_HORIZONTAL: f64 = get_input("FOV in degrees (default: 90): ", "Invalid input. Try again.", 90f64) * PI / 180f64;
    static ref SAMPLES: usize = get_input("Samples per pixel width (default: 1): ", "Invalid input. Try again.", 1usize);

    static ref IMG_SIZE: (u32, u32) = 
    {
        let mut res = String::new();
        loop {
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
        }
    };

    static ref DEGREES_PER_PIXEL: f64 = *FOV_HORIZONTAL / IMG_SIZE.0 as f64;
    static ref FOV_VERTICAL: f64 = *DEGREES_PER_PIXEL * IMG_SIZE.1 as f64;
}

fn main() {
    let _ = (*BACK, *UP, *FOV_HORIZONTAL, *SAMPLES, *IMG_SIZE); //Force order of user input prompts

    let mut threads_available = num_cpus::get() * 2;
    let mut img = RgbImage::new(IMG_SIZE.0, IMG_SIZE.1);

    //drawing pixels
    let stopwatch = Instant::now();

    let (sender, receiver) = mpsc::channel::<(u32, u32, usize, Rgb<u8>)>();
    for j in 0..IMG_SIZE.1
    {
        let rowwatch = Instant::now();
        for i in 0..IMG_SIZE.0
        {
            if threads_available == 0
            {
                let (x, y, free_threads, pixel) = receiver.recv().unwrap();
                img.put_pixel(x, y, pixel);
                threads_available += free_threads;
            }

            let threads_to_use = min(threads_available, *SAMPLES * *SAMPLES);
            threads_available -= threads_to_use;

            let pixel_horizontal_angle = *DEGREES_PER_PIXEL * i as f64 - *FOV_HORIZONTAL / 2f64;
            let pixel_vertical_angle = *DEGREES_PER_PIXEL * j as f64 - *FOV_VERTICAL / 2f64 + *CAMERA_VERTICAL_ANGLE;

            let sender_clone = mpsc::Sender::clone(&sender);
            //pixel thread
            spawn( move || {
                let pixel = get_pixel(pixel_vertical_angle, pixel_horizontal_angle, threads_to_use);
                sender_clone.send((i, j, threads_to_use, pixel)).unwrap();
            });
        }
        println!("{}/{} in {:?}", j+1, IMG_SIZE.1, rowwatch.elapsed());
    }
    drop(sender);
    for message in receiver
    {
        let (x, y, _, pixel) = message;
        img.put_pixel(x, y, pixel);
    }
    println!("Rendered in {:?}", stopwatch.elapsed());

    let time = Local::now();
    img.save(format!("output/black_hole_{}.png", time.format("%Y-%m-%d-%H-%M-%S"))).unwrap();
}

fn get_pixel(vertical_angle: f64, horizontal_angle: f64, threads_to_use: usize) -> Rgb<u8>
{
    let mut colors: Vec<Rgb<u8>> = Vec::new();
    let mut ray_threads_available = threads_to_use;
    let (sender, receiver) = mpsc::channel::<Rgb<u8>>();

    for p_j in 0..*SAMPLES
    {
        for p_i in 0..*SAMPLES
        {
            if ray_threads_available == 0
            {
                colors.push(receiver.recv().unwrap());
                ray_threads_available += 1;
            }
            //ray thread
            let sender_clone = mpsc::Sender::clone(&sender);
            spawn( move || {
                let color = send_ray(horizontal_angle, vertical_angle, p_i, p_j);
                sender_clone.send(color).unwrap();
            });
            ray_threads_available -= 1;
        }
    }
    drop(sender);
    for message in receiver
    {
        colors.push(message);
    }

    //averaging colors
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

fn send_ray(horizontal_angle: f64, vertical_angle: f64, i: usize, j: usize) -> Rgb<u8>
{
    let ray_horizontal_angle = *DEGREES_PER_PIXEL * (i as f64 + 0.5) / *SAMPLES as f64 - *DEGREES_PER_PIXEL / 2f64 + horizontal_angle;
    let ray_vertical_angle = *DEGREES_PER_PIXEL * (j as f64 + 0.5) / *SAMPLES as f64 - *DEGREES_PER_PIXEL / 2f64 + vertical_angle;

    let mut dir_photon: Vector3<f64> = Vector3::new(ray_vertical_angle.cos() * ray_horizontal_angle.sin(), -ray_vertical_angle.sin(), ray_vertical_angle.cos() * ray_horizontal_angle.cos());
    dir_photon = *Unit::new_normalize(dir_photon) * C;
    let mut pos_photon = *CAMERA_POSITION;
    
    let mut color: Rgb<u8> = Rgb([0,0,0]);
    let mut alpha = 0f64;

    let mut delta_time: f64;
    loop
    {
        let dist = pos_photon.magnitude();
        if dist < RS 
        {
            let (c, _) = combine_colors(color, alpha, Rgb([0,0,0]), 1f64);
            return c;
        }
        if dist > RS*20f64
        {
            let c = if (dir_photon.x.abs() > dir_photon.y.abs()) && (dir_photon.x.abs() > dir_photon.z.abs())
            {
                if dir_photon.x > 0f64
                {
                    get_skybox_px(&SKYBOX_RIGHT, dir_photon.x, -dir_photon.z, dir_photon.y)
                }
                else
                {
                    get_skybox_px(&SKYBOX_LEFT, -dir_photon.x, dir_photon.z, dir_photon.y)
                }
            }
            else if dir_photon.y.abs() > dir_photon.z.abs()
            {
                if dir_photon.y > 0f64
                {
                    get_skybox_px(&SKYBOX_TOP, dir_photon.y, dir_photon.x, -dir_photon.z)
                }
                else
                {
                    get_skybox_px(&SKYBOX_BOTTOM, -dir_photon.y, dir_photon.x, dir_photon.z)
                }
            }
            else
            {
                if dir_photon.z > 0f64
                {
                    get_skybox_px(&SKYBOX_FRONT, dir_photon.z, dir_photon.x, dir_photon.y)
                }
                else
                {
                    get_skybox_px(&SKYBOX_BACK, -dir_photon.z, -dir_photon.x, dir_photon.y)
                }
            };

            let (c, _) = combine_colors(color, alpha, c, 1f64);
            return c;
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

        let posvec = new_pos - pos_photon;
        let t = -pos_photon.y/posvec.y;
        let dist = Vector3::new(posvec.x * t + pos_photon.x, 0f64, posvec.z * t + pos_photon.z).magnitude();

        if dist > 3f64*RS && dist < 10f64*RS && (((new_pos.y < 0f64) ^ (pos_photon.y < 0f64)) || (new_pos.y == 0f64) || (pos_photon.y == 0f64))
        {
            let x = ((new_pos.x / RS / 3f64 * *R_ISCO_PX as f64) + (ACCRETION_IMG.width() / 2) as f64) as u32;
            let y = ((new_pos.z / RS / 3f64 * *R_ISCO_PX as f64) + (ACCRETION_IMG.height() / 2) as f64) as u32;
            let intensity = (10f64 - (dist/RS)).sqrt() / 2.82842712475;
            let rgb = ACCRETION_IMG.get_pixel(x, y).to_rgb();
            let (c, a) = combine_colors(color, alpha, rgb, intensity);
            color = c;
            alpha = a;
        }
        pos_photon = new_pos;
    }
}

/*
fn join_pixel_thread(handle: JoinHandle<(u32, u32, usize, Rgb<u8>)>, img: &mut RgbImage) -> usize
{
    let (x, y, threads, rgb) = handle.join().unwrap();
    img.put_pixel(x, y, rgb);
    return threads;
}
*/

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

//alpha is on x, beta is on y
fn get_skybox_px(texture: &RgbImage, main: f64, hori: f64, vert: f64) -> Rgb<u8> 
{
    let sinalpha = hori / (hori * hori + main * main).sqrt();
    let sinbeta = vert / (vert * vert + main * main).sqrt();

    let width = texture.width() - 1;
    let height = texture.height() - 1;
    let x = max(0, min(width, (width as f64 / 2f64 * (1f64 + 1.41421356237 * sinalpha)) as u32));
    let y = max(0, min(height, (height as f64 / 2f64 * (1f64 + 1.41421356237 * sinbeta)) as u32));

    //println!("{}, {}", x, y);

    return texture.get_pixel(x, y).to_rgb();
}

fn combine_colors(color1: Rgb<u8>, alpha1: f64, color2: Rgb<u8>, alpha2: f64) -> (Rgb<u8>, f64)
{
    let alpha2 = (1f64 - alpha1) * alpha2;
    let r = color1[0] as f64 + alpha2 * color2[0] as f64;
    let g = color1[1] as f64 + alpha2 * color2[1] as f64;
    let b = color1[2] as f64 + alpha2 * color2[2] as f64;

    return (Rgb([r as u8, g as u8, b as u8]), alpha1 + alpha2);
}