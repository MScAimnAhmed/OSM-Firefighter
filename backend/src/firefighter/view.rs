extern crate image;

use std::sync::{Arc, RwLock};
use crate::graph::{Graph, GridBounds};

const BLACK: (u8, u8, u8) = (1, 1, 1);
const RED: (u8, u8, u8) = (255, 0, 0);
const BLUE: (u8, u8, u8) = (0, 0, 255);

struct Position {
    lat: f64,
    lon: f64,
}

#[derive(Debug)]
struct ImageBuffer {
    buf: Vec<Vec<(u8, u8, u8)>>,
    width: usize,
    height: usize,
}

impl ImageBuffer {
    fn new(width: usize, height: usize) -> Self {
        Self {
            buf: vec![vec![(0, 0, 0); width]; height],
            width,
            height,
        }
    }

    fn get_px_unchecked(&self, w: usize, h: usize) -> (u8, u8, u8) {
        self.buf[h][w]
    }

    fn set_px_unchecked(&mut self, w: usize, h: usize, px: (u8, u8, u8)) {
        self.buf[h].insert(w, px);
    }
}

pub struct View {
    graph: Arc<RwLock<Graph>>,
    grid_bounds: GridBounds,
    delta_horiz: f64,
    delta_vert: f64,
    img_buf: ImageBuffer,
}

impl View {
    pub fn new(graph: Arc<RwLock<Graph>>, width: usize, height: usize) -> Self {
        let grid_bounds = graph.read().unwrap().get_grid_bounds();
        let delta_horiz = grid_bounds.max_lat - grid_bounds.min_lat;
        let delta_vert = grid_bounds.max_lon - grid_bounds.min_lon;

        let mut view = Self {
            graph,
            grid_bounds,
            delta_horiz,
            delta_vert,
            img_buf: ImageBuffer::new(width, height),
        };

        view.compute_initial();

        view
    }

    fn compute_initial(&mut self) {
        // Initial zoom and center
        let zoom = 1.0;
        let center = Position {
            lat: self.grid_bounds.min_lat + (self.delta_horiz / 2.0),
            lon: self.grid_bounds.min_lon + (self.delta_vert / 2.0),
        };

        // Delta horizontal and vertical depending on zoom
        let d_hz = self.delta_horiz / zoom;
        let d_vert = self.delta_vert / zoom;

        // Grid bounds depending on zoom
        let gb = GridBounds {
            min_lat: center.lat - (d_hz / 2.0),
            max_lat: center.lat + (d_hz / 2.0),
            min_lon: center.lon - (d_vert / 2.0),
            max_lon: center.lon + (d_vert / 2.0),
        };

        // Delta degree per pixel in horizontal and vertical direction
        let deg_per_px_hz = d_hz / self.img_buf.width as f64;
        let deg_per_px_vert = d_vert / self.img_buf.height as f64;

        let graph = self.graph.read().unwrap();

        for h in 0..self.img_buf.height {
            for w in 0..self.img_buf.width {
                // Grid bounds of pixel at w x h
                let min_lat_px = gb.min_lat + w as f64 * deg_per_px_hz;
                let min_lon_px = gb.min_lon + h as f64 * deg_per_px_vert;
                let gb_px = GridBounds {
                    min_lat: min_lat_px,
                    max_lat: min_lat_px + deg_per_px_hz,
                    min_lon: min_lon_px,
                    max_lon: min_lon_px + deg_per_px_vert,
                };

                let mut next = false;
                for node in &graph.nodes {
                    if node.is_located_in(&gb_px) {
                        self.img_buf.set_px_unchecked(w, h, BLACK);
                        next = true;
                        break;
                    }
                }

                if !next {
                    for edge in &graph.edges {
                        let src = &graph.nodes[edge.src];
                        let tgt = &graph.nodes[edge.tgt];
                        if gb_px.intersected_by_segment(src, tgt) {
                            self.img_buf.set_px_unchecked(w, h, BLACK);
                            break;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, RwLock};

    use crate::firefighter::view::{BLACK, View};
    use crate::graph::Graph;

    #[test]
    fn test_view() {
        let graph = Arc::new(RwLock::new(Graph::from_files("data/bbgrund")));
        let view = View::new(graph, 800, 600);

        for row in &view.img_buf.buf {
            for px in row {
                print!("({},{},{})", px.0, px.1, px.2);
            }
            println!();
        }
    }

}