extern crate image;

use std::io::Cursor;
use std::sync::Arc;
use std::cmp::Ordering;

use self::image::{DynamicImage, ImageBuffer, ImageOutputFormat, Rgb, RgbImage};

use crate::firefighter::{problem::NodeDataStorage, TimeUnit};
use crate::graph::{CompassDirection, Graph, GridBounds};

const WHITE: Rgb<u8> = Rgb([255, 255, 255]);
const DARK_GREY: Rgb<u8> = Rgb([64, 64, 64]);
const YELLOW: Rgb<u8> = Rgb([255, 255, 0]);
const RED: Rgb<u8> = Rgb([255, 0, 0]);
const BLUE: Rgb<u8> = Rgb([0, 0, 255]);

/// Type alias for a latitude/longitude tuple
pub type Coords = (f64, f64);

/// Get an `i32` order for an `Rgb<u8>` value
fn get_col_ord(col: &Rgb<u8>) -> i32 {
    match *col {
        DARK_GREY => 0,
        WHITE => 1,
        RED => 2,
        BLUE => 3,
        YELLOW => 4,
        _ => 0
    }
}

/// Compare two `Rgb<u8>` values
fn cmp_col(col1: &Rgb<u8>, col2: &Rgb<u8>) -> Ordering {
    get_col_ord(col1).cmp(&get_col_ord(col2))
}

/// Orientation of an ordered triple of coordinates.
/// # Returns
/// * -1 for counter clockwise
/// * 0 for collinear
/// * 1 for clockwise
fn orientation((lat1, lon1): Coords, (lat2, lon2): Coords, (lat3, lon3): Coords) -> i32 {
    let orientation = (lat2 - lat1) * (lon3 - lon2) -
        (lon2 - lon1) * (lat3 - lat2);

    if orientation < 0.0 { -1 } else if orientation > 0.0 { 1 } else { 0 }
}

/// Line segment between two coordinates
struct LineSegment {
    a: Coords,
    b: Coords,
}

impl LineSegment {
    /// Returns true if this line segment includes the given coordinates
    fn includes(&self, (lat, lon): Coords) -> bool {
        lat <= self.a.0.max(self.b.0) && lat >= self.a.0.min(self.b.0)
            && lon <= self.a.1.max(self.b.1) && lon >= self.a.1.min(self.b.1)
    }

    /// Returns true if this line segment intersects with `other`
    fn intersects(&self, other: &LineSegment) -> bool {
        let o1 = orientation(self.a, self.b, other.a);
        let o2 = orientation(self.a, self.b, other.b);
        let o3 = orientation(other.a, other.b, self.a);
        let o4 = orientation(other.a, other.b, self.b);

        if o1 != o2 && o3 != o4 {
            true
        } else if o1 == 0 && self.includes(other.a) {
            true
        } else if o2 == 0 && self.includes(other.b) {
            true
        } else if o3 == 0 && other.includes(self.a) {
            true
        } else if o4 == 0 && other.includes(self.b) {
            true
        } else {
            false
        }
    }
}

/// View of a specific firefighter simulation
#[derive(Debug)]
pub struct View {
    graph: Arc<Graph>,
    pub(crate) grid_bounds: GridBounds,
    delta_horiz: f64,
    delta_vert: f64,
    img_buf: RgbImage,
    pub initial_center: Coords,
}

impl View {
    /// Create a new firefighter simulation view
    pub fn new(graph: Arc<Graph>, width: u32, height: u32) -> Self {
        let w = if width > 0 { width } else { 1 };
        let h = if height > 0 { height } else { 1 };

        let grid_bounds = graph.get_grid_bounds();
        let delta_horiz = grid_bounds.max_lon - grid_bounds.min_lon;
        let delta_vert = grid_bounds.max_lat - grid_bounds.min_lat;
        let initial_center = (grid_bounds.min_lat + (delta_vert / 2.0),
                             grid_bounds.min_lon + (delta_horiz / 2.0));

        let view = Self {
            graph,
            grid_bounds,
            delta_horiz,
            delta_vert,
            img_buf: ImageBuffer::new(w, h),
            initial_center,
        };

        view
    }

    /// (Re-)compute this view
    pub(super) fn compute(&mut self, center: Coords, zoom: f64, time: &TimeUnit, node_data: &NodeDataStorage) {
        let z = if zoom < 0.0 { 0.0 } else { zoom };

        // Reset view
        for px in self.img_buf.pixels_mut() {
            *px = DARK_GREY;
        }

        // Maximum width and length
        let w_max = (self.img_buf.width() - 1) as i64;
        let h_max = (self.img_buf.height() - 1) as i64;

        // Delta horizontal and vertical depending on zoom
        let d_hz = self.delta_horiz / z;
        let d_vert = self.delta_vert / z;

        // Grid bounds depending on zoom
        let gb = GridBounds {
            min_lat: center.0 - (d_vert / 2.0),
            max_lat: center.0 + (d_vert / 2.0),
            min_lon: center.1 - (d_hz / 2.0),
            max_lon: center.1 + (d_hz / 2.0),
        };

        // Delta degree per pixel in horizontal and vertical direction
        let deg_per_px_hz = d_hz / (w_max+1) as f64;
        let deg_per_px_vert = d_vert / (h_max+1) as f64;

        // For every edge, compute the pixel of its respective source node and iteratively draw the
        // edge until we reach the pixel of the target node
        for edge in self.graph.edges() {
            let src = self.graph.get_node(edge.src);
            let tgt = self.graph.get_node(edge.tgt);

            let mut w_px = ((src.lon - gb.min_lon) / deg_per_px_hz) as i64;
            let mut h_px = ((src.lat - gb.min_lat) / deg_per_px_vert) as i64;

            let ls_edge = LineSegment {
                a: (src.lat, src.lon),
                b: (tgt.lat, tgt.lon),
            };

            let min_lon_px = gb.min_lon + w_px as f64 * deg_per_px_hz;
            let min_lat_px = gb.min_lat + h_px as f64 * deg_per_px_vert;
            let mut gb_px = GridBounds {
                min_lat: min_lat_px,
                max_lat: min_lat_px + deg_per_px_vert,
                min_lon: min_lon_px,
                max_lon: min_lon_px + deg_per_px_hz,
            };

            fn on_north(ls_edge: &LineSegment, gb_px: &mut GridBounds, deg_per_px_vert: f64,
                        h_px: &mut i64) -> bool {
                let ls_px = LineSegment {
                    a: (gb_px.max_lat, gb_px.min_lon),
                    b: (gb_px.max_lat, gb_px.max_lon),
                };
                if ls_edge.intersects(&ls_px) {
                    gb_px.min_lat += deg_per_px_vert;
                    gb_px.max_lat += deg_per_px_vert;
                    *h_px += 1;
                    true
                } else {
                    false
                }
            }

            fn on_east(ls_edge: &LineSegment, gb_px: &mut GridBounds, deg_per_px_hz: f64,
                       w_px: &mut i64) -> bool {
                let ls_px = LineSegment {
                    a: (gb_px.min_lat, gb_px.max_lon),
                    b: (gb_px.max_lat, gb_px.max_lon),
                };
                if ls_edge.intersects(&ls_px) {
                    gb_px.min_lon += deg_per_px_hz;
                    gb_px.max_lon += deg_per_px_hz;
                    *w_px += 1;
                    true
                } else {
                    false
                }
            }

            fn on_south(ls_edge: &LineSegment, gb_px: &mut GridBounds, deg_per_px_vert: f64,
                        h_px: &mut i64) -> bool {
                let ls_px = LineSegment {
                    a: (gb_px.min_lat, gb_px.min_lon),
                    b: (gb_px.min_lat, gb_px.max_lon),
                };
                if ls_edge.intersects(&ls_px) {
                    gb_px.min_lat -= deg_per_px_vert;
                    gb_px.max_lat -= deg_per_px_vert;
                    *h_px -= 1;
                    true
                } else {
                    false
                }
            }

            fn on_west(ls_edge: &LineSegment, gb_px: &mut GridBounds, deg_per_px_hz: f64,
                       w_px: &mut i64) -> bool {
                let ls_px = LineSegment {
                    a: (gb_px.min_lat, gb_px.min_lon),
                    b: (gb_px.max_lat, gb_px.min_lon),
                };
                if ls_edge.intersects(&ls_px) {
                    gb_px.min_lon -= deg_per_px_hz;
                    gb_px.max_lon -= deg_per_px_hz;
                    *w_px -= 1;
                    true
                } else {
                    false
                }
            }

            loop {
                let has_next_px = match tgt.get_relative_compass_direction(&gb_px) {
                    CompassDirection::North => on_north(&ls_edge, &mut gb_px, deg_per_px_vert, &mut h_px),
                    CompassDirection::NorthEast => on_north(&ls_edge, &mut gb_px, deg_per_px_vert, &mut h_px)
                        || on_east(&ls_edge, &mut gb_px, deg_per_px_hz, &mut w_px),
                    CompassDirection::East => on_east(&ls_edge, &mut gb_px, deg_per_px_hz, &mut w_px),
                    CompassDirection::SouthEast => on_east(&ls_edge, &mut gb_px, deg_per_px_hz, &mut w_px)
                        || on_south(&ls_edge, &mut gb_px, deg_per_px_vert, &mut h_px),
                    CompassDirection::South => on_south(&ls_edge, &mut gb_px, deg_per_px_vert, &mut h_px),
                    CompassDirection::SouthWest => on_south(&ls_edge, &mut gb_px, deg_per_px_vert, &mut h_px)
                        || on_west(&ls_edge, &mut gb_px, deg_per_px_hz, &mut w_px),
                    CompassDirection::West => on_west(&ls_edge, &mut gb_px, deg_per_px_hz, &mut w_px),
                    CompassDirection::NorthWest => on_west(&ls_edge, &mut gb_px, deg_per_px_hz, &mut w_px)
                        || on_north(&ls_edge, &mut gb_px, deg_per_px_vert, &mut h_px),
                    CompassDirection::Zero => false
                };

                if !has_next_px {
                    break;
                } else if !gb_px.is_located_in(&gb) {
                    continue;
                }

                self.img_buf.put_pixel(w_px as u32, (h_max - h_px) as u32, WHITE);
            }
        }

        // For every node, compute a circle around its respective pixel and color it
        let mut pxs_to_draw = Vec::with_capacity(self.graph.num_nodes);
        for node in self.graph.nodes() {
            if node.is_located_in(&gb) {
                let w_px = ((node.lon - gb.min_lon) / deg_per_px_hz) as i64;
                let h_px = ((node.lat - gb.min_lat) / deg_per_px_vert) as i64;

                let col_px;
                if node_data.is_root(&node.id) {
                    col_px = YELLOW;
                } else if node_data.is_burning_by(&node.id, time) {
                    col_px = RED;
                } else if node_data.is_defended_by(&node.id, time) {
                    col_px = BLUE;
                } else {
                    col_px = WHITE;
                }

                let r = ((h_max.min(w_max)+1) as f64 * z.log(4.0).max(1.0) / 300.0) as i64;
                pxs_to_draw.reserve((4 * r * r) as usize);
                for w in w_px-r..=w_px+r {
                    for h in h_px-r..=h_px+r {
                        if (((w-w_px).pow(2) + (h-h_px).pow(2)) as f64).sqrt() as i64 <= r {
                            if w >= 0 && w <= w_max && h >= 0 && h <= h_max {
                                pxs_to_draw.push((w as u32, h as u32, col_px));
                            }
                        }
                    }
                }
            }
        }
        pxs_to_draw.sort_unstable_by(|(_, _, col1), (_, _, col2)| cmp_col(col1, col2));
        for (w, h, col) in pxs_to_draw {
            self.img_buf.put_pixel(w, h_max as u32 - h, col);
        }
    }

    /// (Re-)compute this view, using the initial center
    pub(super) fn compute_alt(&mut self, zoom: f64, time: &TimeUnit, node_data: &NodeDataStorage) {
        self.compute(self.initial_center, zoom, time, node_data)
    }

    /// Clones the underlying image buffer, transforms it into a PNG image and returns the image
    /// as raw bytes
    pub fn png_bytes(&self) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(self.img_buf.clone())
            .write_to(&mut buf, ImageOutputFormat::Png)
            .expect("Failed to encode view as PNG image");
        buf.into_inner()
    }

    /// Save the underlying image buffer to a file
    #[allow(dead_code)]
    pub fn save_to_file(&self, path: &str) {
        self.img_buf.save(path).expect("Failed to save view to file");
    }
}