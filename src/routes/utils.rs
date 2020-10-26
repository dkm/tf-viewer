use plotly::{Scatter, Plot, common::Mode};
use staticmap::{Line, StaticMap, Color};
//use crate::{Error};
use anyhow::Result;

pub fn plot(record: &crate::Record) -> Result<String> {

    let heartrate = Scatter::new(record.distance.clone(), record.heartrate.clone())
        .mode(Mode::Lines)
        .name("Heart rate");
    let speed = Scatter::new(record.distance.clone(), record.speed.clone())
        .mode(Mode::Lines)
        .name("Speed");
    let altitude = Scatter::new(record.distance.clone(), record.altitude.clone())
        .mode(Mode::Lines)
        .name("Altitude");

    let mut plot = Plot::new();
    plot.add_trace(heartrate);
    plot.add_trace(speed);
    plot.add_trace(altitude);

    Ok(plot.to_inline_html(None))
}

pub fn generate_thumb(record: crate::Record, id: &str) -> Result<()> {
    if record.lon.is_empty() {
        return Ok(())
    }
    
    let path = format!("static/img/activity/{}.png", id);
    let path = std::path::Path::new(&path);

    if path.exists() {
        return Ok(())
    }

    // Creating file prematurely, preventing more processes from spawning
    // and performing the same task
    std::fs::File::create(&path)?;

    let mut map = StaticMap {
        width: 200,
        height: 200,
        padding: (0, 0), // (x, y)
        x_center: 0.,
        y_center: 0.,
        //url_template: "https://a.tile.openstreetmap.org/%z/%x/%y.png".to_string(),
        url_template: "http://a.tile.komoot.de/komoot-2/%z/%x/%y.png".to_string(),
        tile_size: 256,
        lines: Vec::new(),
        zoom: 0,
    };

    let coordinates: Vec<(f64, f64)> = record.lon
        .into_iter()
        .zip(record.lat)
        .map(|(x, y)|
             if let (Some(a), Some(b)) = (x, y) {
                 (a, b)
             }
             else {
                (0., 0.)
             }
        )
        .collect();

    let line = Line {
        coordinates,
        color: Color {
            r: 255u8,
            g: 0u8,
            b: 0u8,
            a: 255u8,
        },
        width: 6.,
        simplify: true,
    };

    map.add_line(line);

    let image = map.render();
    image.save(path)?;
    Ok(())
}
