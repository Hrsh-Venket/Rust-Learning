use geo::{Polygon, LineString, Coord};
use geo::CoordsIter;
use ordered_float::OrderedFloat;
use std::collections::HashMap;

type Point = (OrderedFloat<f64>, OrderedFloat<f64>);
type Edge = (Point, Point);

fn hertel_mehlhorn(polygon: &Polygon<f64>) -> Vec<Polygon<f64>> {
    // Step 1: Triangulate the polygon
    let mut triangles = triangulate(polygon);
    println!("Step 1: Triangulated polygons:");
    for (i, triangle) in triangles.iter().enumerate() {
        println!("  Triangle {}: {:?}", i + 1, triangle);
    }

    // Step 2: Find all shared edges
    let shared_edges = find_shared_edges(&triangles);
    println!("Step 2: Shared edges with triangles:");
    for (i, (edge, (t1, t2))) in shared_edges.iter().enumerate() {
        println!("  Edge {}: {:?} shared by triangles {} and {}", i + 1, edge, t1, t2);
    }

    // Step 3: Merge triangles into convex polygons
    let mut merged_polygons = triangles.clone();
    let mut to_remove = vec![false; merged_polygons.len()]; // Flags for triangles that get merged.

    for (edge, (t1, t2)) in shared_edges {
        // Skip if either triangle has already been merged
        if to_remove[t1] || to_remove[t2] {
            continue;
        }

        // Merge the two triangles into a single polygon
        let merged_polygon = merge_polygons(&merged_polygons[t1], &merged_polygons[t2], &edge);

        // Check if the merged polygon is convex
        if is_polygon_convex(&merged_polygon) {
            println!("Merging triangles {} and {} into a convex polygon.", t1 + 1, t2 + 1);
        
            // Add the new merged polygon
            merged_polygons.push(merged_polygon);
        
            // Mark t1 and t2 as merged
            to_remove[t1] = true;
            to_remove[t2] = true;
        
            // Add a new entry for the new polygon
            to_remove.push(false);
        } else {
            println!("Triangles {} and {} cannot be merged into a convex polygon.", t1 + 1, t2 + 1);
        }
        
    }

    println!("Length of merged_polygons: {}", merged_polygons.len());
    println!("Length of to_remove: {}", to_remove.len());
    

    // Collect the remaining unmerged polygons
    let final_polygons: Vec<Polygon<f64>> = merged_polygons
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !to_remove[*i])
        .map(|(_, poly)| poly)
        .collect();

    println!("Step 3: Final convex polygons:");
    for (i, poly) in final_polygons.iter().enumerate() {
        println!("  Convex Polygon {}: {:?}", i + 1, poly);
    }

    final_polygons
}

/// Modify `find_shared_edges` to also return the indices of triangles sharing the edges.
fn find_shared_edges(triangles: &[Polygon<f64>]) -> Vec<(Edge, (usize, usize))> {
    let mut edge_map: HashMap<Edge, Vec<usize>> = HashMap::new();

    for (i, triangle) in triangles.iter().enumerate() {
        let coords = triangle.exterior().coords_iter().collect::<Vec<_>>();
        for j in 0..3 {
            let edge = (
                (OrderedFloat(coords[j].x), OrderedFloat(coords[j].y)),
                (OrderedFloat(coords[(j + 1) % 3].x), OrderedFloat(coords[(j + 1) % 3].y)),
            );
            let normalized_edge = if edge.0 < edge.1 { edge } else { (edge.1, edge.0) };
            edge_map.entry(normalized_edge).or_insert_with(Vec::new).push(i);
        }
    }

    edge_map
        .into_iter()
        .filter_map(|(edge, indices)| {
            if indices.len() == 2 {
                Some((edge, (indices[0], indices[1])))
            } else {
                None
            }
        })
        .collect()
}

/// Merge two polygons along a shared edge, preserving anticlockwise order
/// and removing consecutive duplicate points.
fn merge_polygons(p1: &Polygon<f64>, p2: &Polygon<f64>, shared_edge: &Edge) -> Polygon<f64> {
    // Extract exterior coordinates
    let coords1 = p1.exterior().coords_iter().collect::<Vec<_>>();
    let coords2 = p2.exterior().coords_iter().collect::<Vec<_>>();

    // Convert shared edge into coordinates
    let shared_start = Coord {
        x: shared_edge.0 .0.into(),
        y: shared_edge.0 .1.into(),
    };
    let shared_end = Coord {
        x: shared_edge.1 .0.into(),
        y: shared_edge.1 .1.into(),
    };

    // Find the shared edge indices in each polygon
    let shared_idx1 = find_shared_edge(&coords1, shared_start, shared_end);
    let shared_idx2 = find_shared_edge(&coords2, shared_start, shared_end);

    // Reorder polygons to start after the shared edge
    let mut merged_coords = reorder_polygon(&coords1, shared_idx1);
    merged_coords.extend(
        reorder_polygon(&coords2, shared_idx2)
            .into_iter()
            .filter(|&coord| coord != shared_start && coord != shared_end),
    );

    // Close the polygon by appending the starting point
    merged_coords.push(merged_coords[0]);

    // Remove consecutive duplicates
    merged_coords.dedup_by(|a, b| a == b);

    Polygon::new(LineString::from(merged_coords), vec![])
}


/// Find the starting index of the shared edge in a polygon.
fn find_shared_edge(
    coords: &[Coord<f64>],
    shared_start: Coord<f64>,
    shared_end: Coord<f64>,
) -> usize {
    coords
        .windows(2)
        .position(|edge| (edge[0] == shared_start && edge[1] == shared_end)
            || (edge[0] == shared_end && edge[1] == shared_start))
        .expect("Shared edge not found in polygon")
}

/// Reorder the polygon vertices to start after the shared edge.
fn reorder_polygon(coords: &[Coord<f64>], shared_edge_idx: usize) -> Vec<Coord<f64>> {
    let len = coords.len();

    // Handle wrap-around to avoid out-of-bounds access
    let after_shared_edge = if shared_edge_idx + 1 < len {
        &coords[shared_edge_idx + 1..]
    } else {
        &coords[..0] // Empty slice when at the end
    };

    after_shared_edge
        .iter()
        .chain(coords[..=shared_edge_idx].iter())
        .cloned()
        .collect()
}


/// Check if a polygon is convex by ensuring all vertices are convex.
fn is_polygon_convex(polygon: &Polygon<f64>) -> bool {
    let coords = polygon.exterior().coords_iter().collect::<Vec<_>>();
    let len = coords.len();

    // Print out the shape being checked for convexity
    println!("Checking polygon for convexity: {:?}", coords);

    for i in 0..len {
        let prev = coords[(i + len - 1) % len];
        let curr = coords[i];
        let next = coords[(i + 1) % len];
        if !is_convex(prev, curr, next) {
            println!(
                "Polygon is not convex: vertex ({:?}, {:?}, {:?}) forms a concave angle.",
                prev, curr, next
            );
            return false;
        }
    }

    println!("Polygon is convex.");
    true
}


/// Triangulate a simple polygon (no holes) using the ear-clipping algorithm.
fn triangulate(polygon: &Polygon<f64>) -> Vec<Polygon<f64>> {
    let mut coords = polygon.exterior().coords_iter().collect::<Vec<_>>();

    // Check if the polygon is valid for triangulation
    if coords.len() < 4 {
        panic!("Polygon must have at least 4 vertices for triangulation."); // Must be at least a triangle
    }

    // Ensure the polygon is closed (last point == first point)
    if coords.first() == coords.last() {
        coords.pop();
    }

    let mut triangles = Vec::new();

    // Iteratively find and clip ears
    while coords.len() > 3 {
        let mut ear_found = false;

        for i in 0..coords.len() {
            let prev_idx = (i + coords.len() - 1) % coords.len();
            let next_idx = (i + 1) % coords.len();

            let p_prev = coords[prev_idx];
            let p_curr = coords[i];
            let p_next = coords[next_idx];

            // Check if this is a convex vertex
            if is_convex(p_prev, p_curr, p_next) {
                // Check if the triangle formed by these points is an "ear"
                if is_ear(&coords, prev_idx, i, next_idx) {
                    // Create a new triangle
                    let ear = vec![p_prev, p_curr, p_next, p_prev];
                    triangles.push(Polygon::new(LineString::from(ear), vec![]));

                    // Remove the ear vertex from the polygon
                    coords.remove(i);

                    ear_found = true;
                    break;
                }
            }
        }

        if !ear_found {
            panic!("No ears found; the polygon might be invalid or self-intersecting.");
        }
    }

    // Add the final remaining triangle
    let final_triangle = vec![coords[0], coords[1], coords[2], coords[0]];
    triangles.push(Polygon::new(LineString::from(final_triangle), vec![]));

    triangles
}

fn is_convex(p1: Coord<f64>, p2: Coord<f64>, p3: Coord<f64>) -> bool {
    cross_product(p1, p2, p3) >= 0.0
}

/// Compute the cross product of three points.
fn cross_product(p1: Coord<f64>, p2: Coord<f64>, p3: Coord<f64>) -> f64 {
    (p2.x - p1.x) * (p3.y - p2.y) - (p2.y - p1.y) * (p3.x - p2.x)
}

/// Check if a triangle is an "ear" (no other points are inside the triangle).
fn is_ear(coords: &[Coord<f64>], prev_idx: usize, curr_idx: usize, next_idx: usize) -> bool {
    let p1 = coords[prev_idx];
    let p2 = coords[curr_idx];
    let p3 = coords[next_idx];

    for (i, &point) in coords.iter().enumerate() {
        if i == prev_idx || i == curr_idx || i == next_idx {
            continue; // Skip vertices of the current triangle
        }

        if point_in_triangle(point, p1, p2, p3) {
            return false; // If any point is inside the triangle, it is not an ear
        }
    }

    true
}

/// Check if a point is inside a triangle using barycentric coordinates.
fn point_in_triangle(pt: Coord<f64>, v1: Coord<f64>, v2: Coord<f64>, v3: Coord<f64>) -> bool {
    let d1 = cross_product(v1, v2, pt);
    let d2 = cross_product(v2, v3, pt);
    let d3 = cross_product(v3, v1, pt);

    // The point is inside if all cross products have the same sign
    (d1 >= 0.0 && d2 >= 0.0 && d3 >= 0.0) || (d1 <= 0.0 && d2 <= 0.0 && d3 <= 0.0)
}


fn main() {
    // Define a simple polygon

    // let coords = vec![
    //     (4.0, 0.0),
    //     (4.0, 4.0),
    //     (2.0, 3.0),
    //     (0.0, 4.0),
    //     (2.0, 2.0),
    //     (0.0, 0.0),
    //     (4.0, 0.0),
    // ];

    // let coords = vec![
    //     (4.0, 0.0),
    //     (4.0, 4.0),
    //     (0.0, 4.0),
    //     (2.0, 2.0),
    //     (0.0, 0.0),
    //     (4.0, 0.0),
    // ];

    let coords = vec![
        (0.0, 0.0),
        (4.0, 0.0),
        (4.0, 4.0),
        (0.0, 4.0),
        (0.0, 0.0),
    ];

    let polygon = Polygon::new(LineString::from(coords), vec![]);

    // Perform the Hertel-Mehlhorn convex decomposition
    let convex_polygons = hertel_mehlhorn(&polygon);

    // Print the resulting convex polygons
    for (i, convex_polygon) in convex_polygons.iter().enumerate() {
        println!("Convex Polygon {}: {:?}", i + 1, convex_polygon);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{polygon, LineString};

    #[test]
    fn test_simple_square_polygon() {
        let coords = vec![
            (0.0, 0.0),
            (4.0, 0.0),
            (4.0, 4.0),
            (0.0, 4.0),
            (0.0, 0.0),
        ];
        let polygon = Polygon::new(LineString::from(coords), vec![]);
        let result = hertel_mehlhorn(&polygon);

        assert_eq!(result.len(), 1); // A square is already convex
    }

    #[test]
    fn test_concave_polygon() {
        let coords = vec![
            (0.0, 0.0),
            (4.0, 0.0),
            (4.0, 4.0),
            (2.0, 2.0), // Concave point
            (0.0, 4.0),
            (0.0, 0.0),
        ];
        let polygon = Polygon::new(LineString::from(coords), vec![]);
        let result = hertel_mehlhorn(&polygon);

        assert!(result.len() > 1); // Should split into multiple convex polygons
    }

    #[test]
    fn test_triangle_polygon() {
        let coords = vec![
            (0.0, 0.0),
            (4.0, 0.0),
            (2.0, 4.0),
            (0.0, 0.0),
        ];
        let polygon = Polygon::new(LineString::from(coords), vec![]);
        let result = hertel_mehlhorn(&polygon);

        assert_eq!(result.len(), 1); // A triangle is already convex
    }

    #[test]
    fn test_shared_edge_detection() {
        let triangles = vec![
            polygon![(x: 0.0, y: 0.0), (x: 1.0, y: 1.0), (x: 2.0, y: 0.0)],
            polygon![(x: 1.0, y: 1.0), (x: 2.0, y: 0.0), (x: 2.0, y: 2.0)],
        ];
        let shared_edges = find_shared_edges(&triangles);

        assert_eq!(shared_edges.len(), 1); // One shared edge exists
    }

    #[test]
    fn test_convexity_check() {
        let coords_convex = vec![
            (0.0, 0.0),
            (4.0, 0.0),
            (4.0, 4.0),
            (0.0, 4.0),
            (0.0, 0.0),
        ];
        let polygon_convex = Polygon::new(LineString::from(coords_convex), vec![]);
        assert!(is_polygon_convex(&polygon_convex));

        let coords_concave = vec![
            (0.0, 0.0),
            (4.0, 0.0),
            (4.0, 4.0),
            (2.0, 2.0), // Concave point
            (0.0, 4.0),
            (0.0, 0.0),
        ];
        let polygon_concave = Polygon::new(LineString::from(coords_concave), vec![]);
        assert!(!is_polygon_convex(&polygon_concave));
    }
}
