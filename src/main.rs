use image::RgbImage;
use rand::prelude::*;
use std::collections::HashMap;
use std::f64::consts::TAU;
use std::hash::Hash;
use std::time::Instant;

type Location = [usize; 2];
type Color = [u8; 3];
type ColorOffset = [i64; 3];
type LocationOffset = [isize; 2];

// Fraction of circle
#[derive(Debug, Clone, Copy)]
struct Pixel {
    color: Color,
    low_angle: f64,
    high_angle: f64,
}

struct VecSet<T: Copy + Eq + Hash> {
    vec: Vec<T>,
    set: HashMap<T, usize>,
}
impl<T: Copy + Eq + Hash> VecSet<T> {
    fn new() -> Self {
        Self {
            vec: vec![],
            set: HashMap::new(),
        }
    }
    fn insert(&mut self, t: T) {
        let was_new = self.set.insert(t, self.vec.len());
        assert!(was_new.is_none());
        self.vec.push(t);
    }
    fn contains(&self, t: &T) -> bool {
        self.set.contains_key(t)
    }
    fn remove_random<R: Rng>(&mut self, rng: &mut R) -> Option<T> {
        if self.vec.is_empty() {
            return None;
        }
        let index = rng.random_range(0..self.vec.len());
        let t = self.vec.swap_remove(index);
        if index != self.vec.len() {
            self.set.insert(self.vec[index], index);
        }
        self.set.remove(&t);
        Some(t)
    }
    fn remove(&mut self, t: &T) {
        let index = self.set.remove(t).expect("was present");
        self.vec.swap_remove(index);
        if index != self.vec.len() {
            self.set.insert(self.vec[index], index);
        }
    }
}

fn make(size: usize, num_start: usize, angle: f64, seed: u64) -> RgbImage {
    let mut grid: Vec<Vec<Option<Pixel>>> = vec![vec![None; size]; size];
    let mut color_to_location: HashMap<Color, Location> = HashMap::new();
    let mut unfilled: VecSet<Location> = VecSet::new();
    for i in 0..size {
        for j in 0..size {
            unfilled.insert([i, j]);
        }
    }
    let mut color_offsets: Vec<ColorOffset> = (-255..=255)
        .flat_map(|r| (-255..=255).flat_map(move |g| (-255..=255).map(move |b| [r, g, b])))
        .collect();
    color_offsets.sort_by_key(|co| co.map(|c| c.pow(2)).iter().sum::<i64>());
    let mut colors: Vec<Color> = (0..=255)
        .flat_map(|r| (0..=255).flat_map(move |g| (0..=255).map(move |b| [r, g, b])))
        .collect();
    let size_i = size as isize;
    let mut location_offsets: Vec<LocationOffset> = (-size_i..=size_i)
        .flat_map(|i| (-size_i..=size_i).map(move |j| [i, j]))
        .collect();
    location_offsets.sort_by_key(|lo| lo.map(|l| l.pow(2)).iter().sum::<isize>());
    let mut rng = StdRng::seed_from_u64(seed);
    colors.shuffle(&mut rng);
    let mut lo_cache: HashMap<u64, Vec<LocationOffset>> = HashMap::new();
    let lo_cache_cap = 100;
    let start = Instant::now();
    for (num_placed, &color) in colors.iter().enumerate() {
        if num_placed == size * size {
            break;
        }
        if num_placed.is_power_of_two() {
            println!("{num_placed}: {:?}", start.elapsed());
        }
        if num_placed < num_start {
            let location = unfilled.remove_random(&mut rng).expect("enough");
            let low_angle = rng.random();
            let high_angle = (low_angle + angle) % 1.0;
            let pixel = Pixel {
                color,
                low_angle,
                high_angle,
            };
            assert!(grid[location[0]][location[1]].is_none());
            grid[location[0]][location[1]] = Some(pixel);
            let old = color_to_location.insert(color, location);
            assert!(old.is_none());
            continue;
        }
        let new_color = color_offsets
            .iter()
            .filter_map(|co| {
                let new = [
                    color[0] as i64 + co[0] as i64,
                    color[1] as i64 + co[1] as i64,
                    color[2] as i64 + co[2] as i64,
                ];
                if new[0] < 0
                    || new[0] > 255
                    || new[1] < 0
                    || new[1] > 255
                    || new[2] < 0
                    || new[2] > 255
                {
                    None
                } else {
                    let new = new.map(|n| n as u8);
                    if color_to_location.contains_key(&new) {
                        Some(new)
                    } else {
                        None
                    }
                }
            })
            .next()
            .expect("found");
        let location = color_to_location[&new_color];
        let pixel = grid[location[0]][location[1]].expect("present");
        let filtered_los = location_offsets.iter().copied().filter(|&lo| {
            let angle = ((lo[0] as f64).atan2(lo[1] as f64) / TAU) % 1.0;
            if pixel.low_angle < pixel.high_angle {
                pixel.low_angle < angle && angle < pixel.high_angle
            } else {
                pixel.low_angle > angle || angle > pixel.high_angle
            }
        });
        if lo_cache.len() < lo_cache_cap && !lo_cache.contains_key(&pixel.low_angle.to_bits()) {
            lo_cache.insert(pixel.low_angle.to_bits(), filtered_los.clone().collect());
        }
        let maybe_new_location = if let Some(los) = lo_cache.get(&pixel.low_angle.to_bits()) {
            los.iter()
                .filter_map(|lo| {
                    let new = [location[0] as isize + lo[0], location[1] as isize + lo[1]];
                    if new[0] < 0 || new[0] >= size_i || new[1] < 0 || new[1] >= size_i {
                        return None;
                    }
                    let new = new.map(|n| n as usize);
                    if unfilled.contains(&new) {
                        Some(new)
                    } else {
                        None
                    }
                })
                .next()
        } else {
            filtered_los
                .filter_map(|lo| {
                    let new = [location[0] as isize + lo[0], location[1] as isize + lo[1]];
                    if new[0] < 0 || new[0] >= size_i || new[1] < 0 || new[1] >= size_i {
                        return None;
                    }
                    let new = new.map(|n| n as usize);
                    if unfilled.contains(&new) {
                        Some(new)
                    } else {
                        None
                    }
                })
                .next()
        };
        if let Some(new_location) = maybe_new_location {
            let new_low = pixel.low_angle;
            let new_high = (new_low + angle) % 1.0;
            let new_pixel = Pixel {
                color,
                low_angle: new_low,
                high_angle: new_high,
            };
            unfilled.remove(&new_location);
            assert!(grid[new_location[0]][new_location[1]].is_none());
            grid[new_location[0]][new_location[1]] = Some(new_pixel);
            let old = color_to_location.insert(color, new_location);
            assert!(old.is_none());
        } else {
            let new_location = location_offsets.iter().copied().filter_map(|lo| {
                let new = [location[0] as isize + lo[0], location[1] as isize + lo[1]];
                if new[0] < 0 || new[0] >= size_i || new[1] < 0 || new[1] >= size_i {
                    return None;
                }
                let new = new.map(|n| n as usize);
                if unfilled.contains(&new) {
                    Some(new)
                } else {
                    None
                }
            }).next().expect("nonempty");
            let low_angle = rng.random();
            let high_angle = (low_angle + angle) % 1.0;
            let pixel = Pixel {
                color,
                low_angle,
                high_angle,
            };
            unfilled.remove(&new_location);
            assert!(grid[new_location[0]][new_location[1]].is_none());
            grid[new_location[0]][new_location[1]] = Some(pixel);
            let old = color_to_location.insert(color, new_location);
            assert!(old.is_none());
        }
    }
    let mut image = RgbImage::new(size as u32, size as u32);
    for i in 0..size {
        for j in 0..size {
            let color = grid[i][j].expect("filled").color;
            image.put_pixel(i as u32, j as u32, color.into());
        }
    }
    image
}

fn main() {
    let size = 500;
    let num_start = 40;
    let angle = 0.05;
    let seed = 0;
    let filename = format!("img-{}-{}-{}-{}.png", size, num_start, angle, seed);
    println!("{}", filename);
    let image = make(size, num_start, angle, seed);
    image.save(&filename).expect("available");
}
