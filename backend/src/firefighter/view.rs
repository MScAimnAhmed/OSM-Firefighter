extern crate image;

use std::{io::Cursor,
          sync::{Arc, RwLock}};

use self::image::{DynamicImage, ImageBuffer, ImageOutputFormat, Rgb, RgbImage};

use crate::graph::{CompassDirection, Graph, GridBounds};

const WHITE: Rgb<u8> = Rgb([255, 255, 255]);
const BLACK: Rgb<u8> = Rgb([1, 1, 1]);
const RED: Rgb<u8> = Rgb([255, 0, 0]);
const BLUE: Rgb<u8> = Rgb([0, 0, 255]);

/// Type alias for a latitude/longitude tuple
type Coords = (f64, f64);

/// Orientation of an ordered triple of coordinates.
/// # Returns
/// * -1 for counter clockwise
/// * 0 for collinear
/// * 1 for clockwise
fn orientation((lat1, lon1): Coords, (lat2, lon2): Coords, (lat3, lon3): Coords) -> i32 {
    let orientation = (lon2 - lon1) * (lat3 - lat2) -
        (lat2 - lat1) * (lon3 - lon2);

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
    pub fn intersects(&self, other: &LineSegment) -> bool {
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

#[derive(Debug)]
pub struct View {
    graph: Arc<RwLock<Graph>>,
    grid_bounds: GridBounds,
    delta_horiz: f64,
    delta_vert: f64,
    img_buf: RgbImage,
}

impl View {
    pub fn new(graph: Arc<RwLock<Graph>>, width: u32, height: u32) -> Self {
        let w = if width > 0 { width } else { 1 };
        let h = if height > 0 { height } else { 1 };

        let grid_bounds = graph.read().unwrap().get_grid_bounds();
        let delta_horiz = grid_bounds.max_lat - grid_bounds.min_lat;
        let delta_vert = grid_bounds.max_lon - grid_bounds.min_lon;

        let mut view = Self {
            graph,
            grid_bounds,
            delta_horiz,
            delta_vert,
            img_buf: ImageBuffer::from_pixel(w, h, WHITE),
        };

        view.compute_initial();

        view
    }

    fn compute_initial(&mut self) {
        // Initial zoom and center
        let zoom = 1.0;
        let center = (self.grid_bounds.min_lat + (self.delta_horiz / 2.0),
                      self.grid_bounds.min_lon + (self.delta_vert / 2.0));

        // Maximum width and length
        let w_max = self.img_buf.width() - 1;
        let h_max = self.img_buf.height() - 1;

        // Delta horizontal and vertical depending on zoom
        let d_hz = self.delta_horiz / zoom;
        let d_vert = self.delta_vert / zoom;

        // Grid bounds depending on zoom
        let gb = GridBounds {
            min_lat: center.0 - (d_hz / 2.0),
            max_lat: center.0 + (d_hz / 2.0),
            min_lon: center.1 - (d_vert / 2.0),
            max_lon: center.1 + (d_vert / 2.0),
        };

        // Delta degree per pixel in horizontal and vertical direction
        let deg_per_px_hz = d_hz / (w_max+1) as f64;
        let deg_per_px_vert = d_vert / (h_max+1) as f64;

        let graph = self.graph.read().unwrap();

        // For every node, compute its respective pixel and color it
        for node in &graph.nodes {
            if node.is_located_in(&gb) {
                let mut w_px = ((node.lat - gb.min_lat) / deg_per_px_hz) as i64;
                let mut h_px = ((node.lon - gb.min_lon) / deg_per_px_vert) as i64;
                if w_px > w_max as i64 {
                    w_px = w_max as i64
                }
                if h_px > h_max as i64 {
                    h_px = h_max as i64
                }

                let r = ((h_max+1) as f64 * zoom / 300.0) as i64;
                for w in w_px-r..=w_px+r {
                    for h in h_px-r..=h_px+r {
                        if (((w-w_px).pow(2) + (h-h_px).pow(2)) as f64).sqrt() as i64 <= r {
                            if w >= 0 && w <= w_max as i64 && h >= 0 && h <= h_max as i64 {
                                self.img_buf.put_pixel(w as u32, h as u32, BLACK);
                            }
                        }
                    }
                }
            }
        }

        // For every edge, compute the pixel of its respective source node and iteratively draw the
        // edge until we reach the pixel of the target node
        for edge in &graph.edges {
            let src = &graph.nodes[edge.src];
            let tgt = &graph.nodes[edge.tgt];

            let mut w_px = ((src.lat - gb.min_lat) / deg_per_px_hz) as i32;
            let mut h_px = ((src.lon - gb.min_lon) / deg_per_px_vert) as i32;
            if w_px > w_max as i32 {
                w_px = w_max as i32
            }
            if h_px > h_max as i32 {
                h_px = h_max as i32
            }

            let ls_edge = LineSegment {
                a: (src.lat, src.lon),
                b: (tgt.lat, tgt.lon),
            };

            let min_lat_px = gb.min_lat + w_px as f64 * deg_per_px_hz;
            let min_lon_px = gb.min_lon + h_px as f64 * deg_per_px_vert;
            let mut gb_px = GridBounds {
                min_lat: min_lat_px,
                max_lat: min_lat_px + deg_per_px_hz,
                min_lon: min_lon_px,
                max_lon: min_lon_px + deg_per_px_vert,
            };

            fn on_north(ls_edge: &LineSegment, gb_px: &mut GridBounds, deg_per_px_vert: f64,
                        h_px: &mut i32) -> bool {
                let ls_px = LineSegment {
                    a: (gb_px.min_lat, gb_px.max_lon),
                    b: (gb_px.max_lat, gb_px.max_lon),
                };
                if ls_edge.intersects(&ls_px) {
                    gb_px.min_lon += deg_per_px_vert;
                    gb_px.max_lon += deg_per_px_vert;
                    *h_px += 1;
                    true
                } else {
                    false
                }
            }

            fn on_east(ls_edge: &LineSegment, gb_px: &mut GridBounds, deg_per_px_hz: f64,
                       w_px: &mut i32) -> bool {
                let ls_px = LineSegment {
                    a: (gb_px.max_lat, gb_px.min_lon),
                    b: (gb_px.max_lat, gb_px.max_lon),
                };
                if ls_edge.intersects(&ls_px) {
                    gb_px.min_lat += deg_per_px_hz;
                    gb_px.max_lat += deg_per_px_hz;
                    *w_px += 1;
                    true
                } else {
                    false
                }
            }

            fn on_south(ls_edge: &LineSegment, gb_px: &mut GridBounds, deg_per_px_vert: f64,
                        h_px: &mut i32) -> bool {
                let ls_px = LineSegment {
                    a: (gb_px.min_lat, gb_px.min_lon),
                    b: (gb_px.max_lat, gb_px.min_lon),
                };
                if ls_edge.intersects(&ls_px) {
                    gb_px.min_lon -= deg_per_px_vert;
                    gb_px.max_lon -= deg_per_px_vert;
                    *h_px -= 1;
                    true
                } else {
                    false
                }
            }

            fn on_west(ls_edge: &LineSegment, gb_px: &mut GridBounds, deg_per_px_hz: f64,
                       w_px: &mut i32) -> bool {
                let ls_px = LineSegment {
                    a: (gb_px.min_lat, gb_px.min_lon),
                    b: (gb_px.min_lat, gb_px.max_lon),
                };
                if ls_edge.intersects(&ls_px) {
                    gb_px.min_lat -= deg_per_px_hz;
                    gb_px.max_lat -= deg_per_px_hz;
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

                if !has_next_px || tgt.is_located_in(&gb_px) {
                    break;
                } else if !gb_px.is_located_in(&gb) {
                    continue;
                }

                self.img_buf.put_pixel(w_px as u32, h_px as u32, BLACK);
            }
        }
    }

    /// Clones the underlying image buffer, transforms it into a PNG image and returns the image
    /// as raw bytes
    pub fn png_bytes(&self) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(self.img_buf.clone())
            .write_to(&mut buf, ImageOutputFormat::Png)
            .unwrap();
        buf.into_inner()
    }

    /// Save the underlying image buffer to a file
    fn save_to_file(&self, path: &str) {
        self.img_buf.save(path).unwrap();
    }
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, RwLock};

    use crate::firefighter::view::View;
    use crate::graph::Graph;

    #[test]
    fn test_view() {
        let graph = Arc::new(RwLock::new(Graph::from_files("data/stgcenter")));
        let view = View::new(graph, 1920, 1080);

        for px in view.img_buf.pixels() {
            // TODO check pixel
        }

        view.save_to_file("data/stgcenter.png");
    }

}