#![deny(clippy::all)]

use napi_derive::napi;

#[napi]
pub fn plus_100(input: u32) -> u32 {
  input + 100
}

// Wewnętrzna funkcja dekodująca Google Polyline (odpowiednik polyline.decode z JS)
fn decode_polyline(encoded: &str) -> Vec<(f64, f64)> {
  let mut index = 0;
  let mut lat: i32 = 0;
  let mut lon: i32 = 0;
  let mut res = Vec::new();
  let bytes = encoded.as_bytes();
  let factor = 100_000.0; // Precyzja 5 (10^5)

  while index < bytes.len() {
    let mut b;
    let mut shift = 0;
    let mut result = 0;
    loop {
      if index >= bytes.len() {
        break;
      }
      b = bytes[index] as i32 - 63;
      index += 1;
      result |= (b & 0x1f) << shift;
      shift += 5;
      if b < 0x20 {
        break;
      }
    }
    let dlat = if (result & 1) != 0 {
      !(result >> 1)
    } else {
      result >> 1
    };
    lat += dlat;

    shift = 0;
    result = 0;
    loop {
      if index >= bytes.len() {
        break;
      }
      b = bytes[index] as i32 - 63;
      index += 1;
      result |= (b & 0x1f) << shift;
      shift += 5;
      if b < 0x20 {
        break;
      }
    }
    let dlon = if (result & 1) != 0 {
      !(result >> 1)
    } else {
      result >> 1
    };
    lon += dlon;

    // Tablica przechowuje krotki: (latitude, longitude)
    res.push((lat as f64 / factor, lon as f64 / factor));
  }
  res
}

#[napi]
pub fn is_point_on_line_fast(
  encoded_polyline: String,
  p_lat: f64,
  p_lon: f64,
  margin: f64,
) -> bool {
  let points = decode_polyline(&encoded_polyline);
  if points.is_empty() {
    return false;
  }

  // Przeliczniki stopni na metry
  let lat_to_meters = 111320.0;
  let lon_to_meters = 111320.0 * (p_lat * std::f64::consts::PI / 180.0).cos();

  let margin_lat = margin / lat_to_meters;
  let margin_lon = margin / lon_to_meters;
  let margin_sq = margin * margin;

  // Iteracja po segmentach (oknach po 2 punkty)
  for window in points.windows(2) {
    let p1 = window[0];
    let p2 = window[1];

    // 1. Bounding Box Check (Błyskawiczne odrzucanie dalekich segmentów)
    let min_lat = p1.0.min(p2.0) - margin_lat;
    let max_lat = p1.0.max(p2.0) + margin_lat;
    if p_lat < min_lat || p_lat > max_lat {
      continue;
    }

    let min_lon = p1.1.min(p2.1) - margin_lon;
    let max_lon = p1.1.max(p2.1) + margin_lon;
    if p_lon < min_lon || p_lon > max_lon {
      continue;
    }

    // 2. Rzutowanie na lokalną płaszczyznę w metrach (cel to punkt 0,0)
    let x1 = (p1.1 - p_lon) * lon_to_meters;
    let y1 = (p1.0 - p_lat) * lat_to_meters;
    let x2 = (p2.1 - p_lon) * lon_to_meters;
    let y2 = (p2.0 - p_lat) * lat_to_meters;

    // Obliczanie odległości punktu od odcinka
    let l2 = (x2 - x1) * (x2 - x1) + (y2 - y1) * (y2 - y1);
    let dist_sq;

    if l2 == 0.0 {
      // Segment jest pojedynczym punktem
      dist_sq = x1 * x1 + y1 * y1;
    } else {
      // Rzutowanie punktu na odcinek z użyciem metody clamp zamiast Math.max/min
      let t = (((-x1) * (x2 - x1) + (-y1) * (y2 - y1)) / l2).clamp(0.0, 1.0);

      let proj_x = x1 + t * (x2 - x1);
      let proj_y = y1 + t * (y2 - y1);
      dist_sq = proj_x * proj_x + proj_y * proj_y;
    }

    // 3. Early Exit (Wczesne wyjście)
    if dist_sq <= margin_sq {
      return true;
    }
  }

  // Obsługa rzadkiego przypadku: trasa z 1 punktem
  if points.len() == 1 {
    let x1 = (points[0].1 - p_lon) * lon_to_meters;
    let y1 = (points[0].0 - p_lat) * lat_to_meters;
    if x1 * x1 + y1 * y1 <= margin_sq {
      return true;
    }
  }

  false
}
