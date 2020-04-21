use nalgebra::{Rotation3, Vector3, Unit};
use image::{RgbImage, Rgb};
use std::cmp::{max, min};
use std::f64::consts::PI;

const C: f64 = 299792458.0;
const M: f64 = 8.3e36;
const G: f64 = 6.6743e-11;

fn main() {
    let rs = 2f64 / 3f64 * G * M / C / C /*wtf?*/;
    let scale = 100f64 / rs;
    let pos_blackhole: Vector3<f64> = Vector3::new(500f64,500f64,0f64) / scale;

    let mut img = image::RgbImage::new(1000, 1000);

    let y = 174f64;
    let spacing = 0.1f64;

    for i in 0..100
    {
        send_ray(&mut img, &pos_blackhole, scale, Vector3::new(1000f64, y + i as f64 * spacing, 0f64)/scale, Vector3::new(-1f64, 0f64, 0f64));
    }

    draw_circle(&mut img, (pos_blackhole.x * scale) as u32, (pos_blackhole.y * scale) as u32, (rs * scale) as u32, Rgb([0, 0, 255]));

    img.save("image.png").unwrap();
}

fn send_ray(img: &mut RgbImage, pos_blackhole: &Vector3<f64>, scale: f64, mut pos_photon: Vector3<f64>, mut dir_photon: Vector3<f64>)
{
    dir_photon = *Unit::new_normalize(dir_photon) * C;

    let delta_time = 0.01f64;

    for _i in 0..100000
    {
        let dist_vec = pos_blackhole - pos_photon;
        let dist: f64 = dist_vec.magnitude();
        let force = G * M / dist / dist;
        let delta_theta = (PI - dist_vec.angle(&dir_photon)).sin() * force / C * delta_time;
        let u = dir_photon.cross(&dist_vec);

        let rotation_matrix = Rotation3::from_axis_angle(&Unit::new_normalize(u), delta_theta);
        dir_photon = rotation_matrix * dir_photon;

        let new_pos = pos_photon + dir_photon * delta_time;

        if !(pos_photon.x < 0.0 || pos_photon.y < 0.0 || new_pos.x < 0.0 || new_pos.y < 0.0)
        {
            draw_line(img, (pos_photon.x * scale) as u32, (pos_photon.y * scale) as u32, (new_pos.x * scale) as u32, (new_pos.y * scale) as u32, Rgb([255, 255, 0]));
        }
        pos_photon = new_pos;
    }
}

fn draw_circle(img: &mut RgbImage, x: u32, y: u32, r: u32, rgb: Rgb<u8>) {
    for i in max(0, x as i32 - r as i32) as u32 .. min(img.width(), x + r + 1)
    {
        for j in max(0, y as i32 - r as i32) as u32 .. min(img.height(), y + r + 1)
        {
            if (((i as i32 - x as i32).pow(2) + (j as i32 - y as i32).pow(2)) as f32).sqrt() <= r as f32
            {
                img.put_pixel(i, j, rgb);
            }
        }
    }
}

fn draw_line(img: &mut RgbImage, x1: u32, y1: u32, x2: u32, y2: u32, rgb: Rgb<u8>)
{
    let l = (x2 as f64 - x1 as f64 + y2 as f64 - y1 as f64).abs();

    for i in 0..(l as u32 + 1)
    {
        let x = ((x1 as f64 * i as f64 + x2 as f64 * (l - i as f64)) / l) as u32;
        let y = ((y1 as f64 * i as f64 + y2 as f64 * (l - i as f64)) / l) as u32;
        draw_circle(img, x, y, 1, rgb);
        //img.put_pixel((x1 * i + x2 * (l - i)) / l, (y1 * i + y2 * (l - i)) / l, rgb);
    }
}