extern crate image;

use std::sync::{Arc, RwLock};
use image::ImageBuffer;
use image::RgbImage;
use crate::graph::{Graph, GridBounds};
use self::image::Rgb;

const BLACK: Rgb<u8> = Rgb([0, 0, 0]);
const RED: Rgb<u8> = Rgb([255, 0, 0]);
const BLUE: Rgb<u8> = Rgb([0, 0, 255]);

struct Position {
    lat: f64,
    lon: f64,
}

pub struct View {
    graph: Arc<RwLock<Graph>>,
    grid_bounds: GridBounds,
    delta_horiz: f64,
    delta_vert: f64,
    img_buf: RgbImage,
}

impl View {
    pub fn new(graph: Arc<RwLock<Graph>>, width: u32, height: u32) -> Self {
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
        let deg_per_px_hz = d_hz / self.img_buf.width() as f64;
        let deg_per_pix_vert = d_vert / self.img_buf.height() as f64;

        let graph = self.graph.read().unwrap();

        for w in 0..self.img_buf.width() {
            for h in 0..self.img_buf.height() {
                // Grid bounds of pixel at w x h
                let min_lat_px = gb.min_lat + w * deg_per_px_hz;
                let min_lon_px = gb.min_lon + h * deg_per_pix_vert;
                let gb_px = GridBounds {
                    min_lat: min_lat_px,
                    max_lat: min_lat_px + deg_per_px_hz,
                    min_lon: min_lon_px,
                    max_lon: min_lon_px + deg_per_pix_vert,
                };

                // Nodes and edges located or partly located in pixel at w x h
                let nodes_px: Vec<_> = graph.nodes.iter()
                    .filter(|&n| n.is_located_in(&gb_px))
                    .collect();
                let mut edges_px = Vec::new();
                for n in nodes_px {
                    edges_px.reserve(graph.get_out_degree(n.id));
                    for i in graph.offsets[n.id]..graph.offsets[n.id+1] {
                        let edge = &graph.edges[i];
                        edges_px.push(edge);
                    }
                }
            }
        }
    }
}