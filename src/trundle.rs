use std::{
    error::Error,
    fs::File,
    io::{self, Read, BufReader},
    thread,
    time::{SystemTime, Duration},
};
use lazy_static::lazy_static;
use png::Decoder;
use x11rb::{
    connection::Connection,
    errors::ConnectionError,
    protocol::{
        Event,
        xproto::{*, GX},
    },
    wrapper::ConnectionExt as _,
    COPY_DEPTH_FROM_PARENT,
};

pub const HIGHBACKGROUND_COLOUR: usize = 0;
pub const LOWBACKGROUND_COLOUR: usize = 1;
pub const HIGHLIGHT_COLOUR: usize = 2;
pub const LOWLIGHT_COLOUR: usize = 3;
pub const WALLPAPER_COLOUR: usize = 4;
pub const TITLEBAR_COLOUR: usize = 5;

lazy_static! {
	pub static ref COLOURS: Vec<Option<u32>> = loadcolours("colours.txt", [
        0xBBBBBB, //HIGHBACKGROUND_COLOUR
        0x888888, //LOWBACKGROUND_COLOUR
        0xFFFFFF, //HIGHLIGHT_COLOUR
        0x000000, //LOWLIGHT_COLOUR
        0x008080, //WALLPAPER_COLOUR
		0x0000A8, //TITLEBAR_COLOUR
    ]);
	
    //let mut depressedborder = Element {
    //    command: vec![(1, 255, 0), (2, 128, 1)],
    //    coordinates: vec![(0, 21), (30, 40)],
    //};
	
}

pub fn squishtext(text: &str, width: i16, length: i16) -> String {
    if width < length + 2 {
        String::new()
    } else {
        let characters = ((width - 2) / length) as usize;
        
        if text.len() <= characters {
            text.to_string()
        } else if characters > 3 {
            format!("{}...", &text[0..(characters - 3)])
        } else if characters > 0 {
            text[0..characters].to_string()
        } else {
            String::new()
        }
    }
}


pub fn loadcolours<const N: usize>(file_path: &str, default: [u32; N]) -> Vec<Option<u32>> {
    match std::fs::read_to_string(file_path) {
        Ok(contents) => {
            let mut colors = Vec::with_capacity(default.len());
            for (i, line) in contents.lines().filter(|line| !line.trim().is_empty()).take(default.len()).enumerate() {
                let trimmed = line.trim();
                let hex = trimmed.strip_prefix("0x").unwrap_or(trimmed);
                match u32::from_str_radix(hex, 16) {
                    Ok(value) => colors.push(Some(value)),
                    Err(_) => colors.push(Some(default[i])),
                }
            }
            //If file is sort, fill the rest of the array with the defaults.
            while colors.len() < default.len() {
                colors.push(Some(default[colors.len()]));
            }
            colors
        },
        Err(_) => default.iter().map(|&c| Some(c)).collect(),
    }
}

pub fn drawtitlebar<C: Connection>(xconnection: &C, window: u32, width: i16, height: i16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32, gc_titlebar: u32) -> Result<(), Box<dyn Error>> {

	
	xconnection.poly_fill_rectangle(window, gc_titlebar, &[Rectangle {x: 4, y: 4, width: width as u16, height: height as u16,}])?;
	
	drawbumpyframe(&xconnection, window, width - 14, 6, 15, 13, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground)?;
	drawbumpyframe(&xconnection, window, width - 30, 6, 15, 13, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground)?;
	drawbumpyframe(&xconnection, window, width - 46, 6, 15, 13, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground)?;
	//println!("{}", width - 14);
	
    Ok(())
}

pub fn grabwindowtitle<C: x11rb::connection::Connection>(xconnection: &C, window: u32,) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let reply = xconnection.get_property(false, window, AtomEnum::WM_NAME, AtomEnum::STRING, 0, 1024)?.reply()?;
    if reply.value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(String::from_utf8_lossy(&reply.value).to_string()))
    }
}

pub fn drawtitletext<C: x11rb::connection::Connection>(xconnection: &C, drawable: u32, gc: u32, window: u32, x: i16, y: i16,) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(title) = grabwindowtitle(xconnection, window)? {
        xconnection.image_text8(drawable, gc, x, y, title.as_bytes())?;
    }
    Ok(())
}

pub fn windowborder<C: Connection>(xconnection: &C, window: u32, width: i16, height: i16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32) -> Result<(), Box<dyn Error>> {
	//Border background!
	xconnection.poly_fill_rectangle(window, gc_highbackground, &[Rectangle {x: 0, y: 0, width: width as u16, height: height as u16,}])?;
	//Border highlight!
	xconnection.poly_line(CoordMode::PREVIOUS, window, gc_highlight, &[
		Point { x: width - 3, y: 1 },
		Point { x: 4 - width, y: 0 },
		Point { x: 0, y: height - 4 },
	])?;
	xconnection.poly_line(CoordMode::PREVIOUS, window, gc_lowbackground, &[
		Point { x: 1, y: height - 2 },
		Point { x: width - 3, y: 0 },
		Point { x: 0, y: 3 - height },
	])?;
	xconnection.poly_line(CoordMode::PREVIOUS, window, gc_lowlight, &[
		Point { x: width - 1, y: 0 },
		Point { x: 0, y: height - 1 },
		Point { x: 1 - width, y: 0 },
	])?;
    Ok(())
}

pub fn drawstartbutton<C: Connection>(xconnection: &C, window: u32, startx: i16, starty: i16, framewidth: i16, frameheight: i16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32) -> Result<(), Box<dyn Error>> {

	drawbumpyframe(&xconnection, window, startx, starty, framewidth, frameheight, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground)?;
	drawpng(&xconnection, window, "computer.png", startx + 4, 7, 16, 16, COLOURS[HIGHBACKGROUND_COLOUR])?;
	
	xconnection.image_text8(window, gc_lowlight, startx+22, 19, "S".as_bytes());
	xconnection.image_text8(window, gc_lowlight, startx+22+5, 19, "t".as_bytes());
	xconnection.image_text8(window, gc_lowlight, startx+22+5+5, 19, "ar".as_bytes());
	xconnection.image_text8(window, gc_lowlight, startx+22+5+5+11, 19, "t".as_bytes());
    Ok(())
}

pub fn drawdepressedbumpyframe<C: Connection>(xconnection: &C, window: u32, startx: i16, starty: i16, framewidth: i16, frameheight: i16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32, gc_highcheckers: u32) -> Result<(), Box<dyn Error>> {
	drawbumpyframe(&xconnection, window, startx, starty, framewidth, frameheight, gc_lowlight, gc_highlight, 0, gc_highbackground)?;
	
	xconnection.poly_line(CoordMode::PREVIOUS, window, gc_lowbackground, &[
		Point { x: startx + 1, y: starty + frameheight - 2 },
		Point { x: 0, y: 3 - frameheight },
		Point { x: framewidth - 3, y: 0 },
	])?;
	
	if gc_highcheckers > 0 {
	
		xconnection.poly_line(CoordMode::PREVIOUS, window, gc_highlight, &[
			Point { x: startx + 2, y: 6 },
			Point { x: framewidth - 4, y: 0 },
		])?;
	
	
		xconnection.poly_fill_rectangle(window, gc_highcheckers, &[Rectangle { x: startx + 2, y: starty + 3, width: framewidth as u16 - 3, height: frameheight as u16 - 4}])?;
	}
	
    Ok(())
}



pub fn drawbumpyframe<C: Connection>(xconnection: &C, window: u32, startx: i16, starty: i16, framewidth: i16, frameheight: i16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32) -> Result<(), Box<dyn Error>> {
	//Frame that dumps out.
	xconnection.poly_line(CoordMode::PREVIOUS, window, gc_lowlight, &[
		Point { x: startx, y: starty + frameheight },
		Point { x: framewidth, y: 0 },
		Point { x: 0, y: -frameheight },
	])?;
	xconnection.poly_line(CoordMode::PREVIOUS, window, gc_highlight, &[
		Point { x: startx, y: starty + frameheight - 1 },
		Point { x: 0, y: 1 - frameheight },
		Point { x: framewidth - 1, y: 0 },
	])?;
	xconnection.poly_line(CoordMode::PREVIOUS, window, gc_lowbackground, &[
		Point { x: startx + 1, y: starty + frameheight - 1 },
		Point { x: framewidth - 2, y: 0 },
		Point { x: 0, y: 2-frameheight },
	])?;
	if gc_highbackground > 0 {
		xconnection.poly_fill_rectangle(window, gc_highbackground, &[Rectangle {x: startx + 1, y: starty + 1, width: (framewidth as u16) - 2, height: (frameheight as u16) - 2,}])?;
    }
	Ok(())
}

pub fn drawdepressedframe<C: Connection>(xconnection: &C, window: u32, startx: i16, starty: i16, framewidth: i16, frameheight: i16, gc_highlight: u32, gc_lowbackground: u32) -> Result<(), Box<dyn Error>> {
	//Draw a depressed frame around the target. For example, the notification area.
	xconnection.poly_line(CoordMode::PREVIOUS, window, gc_highlight, &[
		Point { x: startx, y: starty },
		Point { x: 0, y: frameheight },
		Point { x: -framewidth, y: 0 },
	])?;
	xconnection.poly_line(CoordMode::PREVIOUS, window, gc_lowbackground, &[
		Point { x: startx - 1, y: starty },
		Point { x: -framewidth+1, y: 0 },
		Point { x: 0, y: frameheight - 1 },
	])?;
    Ok(())
}

pub fn drawclock<C: Connection>(xconnection: &C, window: u32, gc_lowlight: u32, width: i16, clockheight: i16) -> Result<(u64, u8, u8), Box<dyn Error>> {
    //Clock separator.
    xconnection.poly_point(CoordMode::PREVIOUS, window, gc_lowlight, &[ 
        Point { x: width - 42, y: clockheight - 2 }, 
        Point { x: 0, y: -4 } 
    ])?;
    
    let epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    let mut pminute = ((epoch / 60) % 60) as u8;
    let mut phour = ((epoch / 3600) % 24) as u8;
    let mut chour = phour;
    
    if phour > 12 {
        chour -= 12;
    }
    if phour == 0 {
        chour = 12;
    }
    
    //Hour side padding... this'll need fixing.
    let thour = if chour < 10 { //TODO: We probably want to adjust the width, rather than enter a space.
        format!(" {}", chour)
    } else {
        chour.to_string()
    };
    
    //AM or PM!
    if phour < 12 { 
        xconnection.image_text8(window, gc_lowlight, width - 25, clockheight, "AM".as_bytes())?;
    } else {
        xconnection.image_text8(window, gc_lowlight, width - 25, clockheight, "PM".as_bytes())?;
    };
    
    //Hours!
    xconnection.image_text8(window, gc_lowlight, width - 55, clockheight, thour.as_bytes())?;
    
    //println!("{}:{}", phour, pminute);

	(phour, pminute) = updateclock(&xconnection, window, gc_lowlight, phour, 255, pminute, width, clockheight)?;
    
    Ok((epoch, pminute, phour))
}

pub fn updateclock<C: Connection>(xconnection: &C, window: u32, gc: u32, mut phour: u8, pminute: u8, cminute: u8, width: i16, height: i16) -> Result<(u8, u8), Box<dyn Error>> {
	//println!("UpdateClock Called!");
	if cminute != pminute {
		//println!("cminute != pminute");
		//Draw minute.
		let mut tminute = cminute.to_string();
		if cminute < 10 {
			tminute = format!("0{}", tminute);
		}
		xconnection.image_text8(window, gc, width - 40, height, tminute.as_bytes());
		if cminute == 0 {
			//Draw hour.
			phour = phour + 1;
			if phour > 24 {
				phour = 0;
				//Draw AM.
				xconnection.image_text8(window, gc, width - 25, height, "AM".as_bytes());
			} else {
				//Draw PM.
				xconnection.image_text8(window, gc, width - 25, height, "PM".as_bytes());
			}
			xconnection.image_text8(window, gc, width - 55, height, phour.to_string().as_bytes());
		}
		//println!("tminute {}", tminute);
	}
    Ok((phour, cminute))
}


pub fn drawbookerframe<C: Connection>(xconnection: &C, window: u32, startx: i16, starty: i16, framewidth: i16, frameheight: i16, thickness: i16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32) -> Result<(), Box<dyn Error>> {
	//This frame is a little more complicated.
	//Optionally, draw a border of-1 on the top, and left, and +1 on the bottom and right.
	if gc_lowlight > 0 {
		xconnection.poly_line(CoordMode::PREVIOUS, window, gc_lowlight, &[
			Point { x: startx, y: starty - 1 },
			Point { x: framewidth - 1, y: 0 },
			Point { x: 1, y: 1 },
			Point { x: 0, y: frameheight - 1},
			Point { x: -1, y: 1 },
			Point { x: 1 - framewidth, y: 0 },
			Point { x: -1, y: -1 },
			Point { x: 0, y: 1 - frameheight},
		])?;
	}
	//Start with a light background colour for the width and hight starting at startx and starty.
	
    let endx = startx + framewidth;
    let endy = starty + frameheight;
    let innerwidth = framewidth - thickness;
    let innerheight = frameheight - thickness;
    
    let mut rects = Vec::with_capacity(3);
    let mut rectgcs = Vec::with_capacity(3);
	
    if gc_highbackground > 0 {
        rects.push(Rectangle {x: startx + thickness, y: starty + thickness, width: innerwidth as u16, height: innerheight as u16});
        rectgcs.push(gc_highbackground);
    }

	//Draw two rectangles on the bottom and right using colour2 of defined thickness.
    if gc_lowbackground > 0 {
        rects.push(Rectangle {x: endx - thickness, y: starty, width: thickness as u16, height: frameheight as u16});
        rectgcs.push(gc_lowbackground);
        rects.push(Rectangle {x: startx, y: endy - thickness, width: innerwidth as u16, height: thickness as u16});
        rectgcs.push(gc_lowbackground);
    }
	
    if !rects.is_empty() {
        let mut groups: std::collections::HashMap<u32, Vec<Rectangle>> = std::collections::HashMap::new();
        
        for (rect, gc) in rects.into_iter().zip(rectgcs.into_iter()) {
            groups.entry(gc).or_insert_with(Vec::new).push(rect);
        }
        
        for (gc, rectlist) in groups {
            xconnection.poly_fill_rectangle(window, gc, &rectlist)?;
        }
    }
	
	//Draw lines up to defined thickness of colour1 on the top and left.
	
	//Make loop for index, 0 to thickness
    if gc_highlight > 0 && thickness > 0 {
        let mut points = Vec::with_capacity((thickness as usize) * 3);
        
        for index in 0..thickness {
            points.extend_from_slice(&[
                Point { x: endx - 2 - index, y: starty + index },
                Point { x: index + index + 2 - framewidth, y: 0 },
                Point { x: 0, y: frameheight - 2 - index - index },
            ]);
        }
        
        for chunk in points.chunks(3) {
            xconnection.poly_line(CoordMode::PREVIOUS, window, gc_highlight, chunk)?;
        }
    }

	Ok(())
}

pub fn drawradiobutton<C: Connection>(xconnection: &C, window: u32, startx: i16, starty: i16, size: i16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32) -> Result<(), Box<dyn Error>> {
	//Draw radio button with diameter of size.
    const ANGLE45: i16 = 2880;
    const ANGLE180: i16 = 11520;
    const ANGLE225: i16 = 14400;
    const ANGLE360: i16 = 23040;
    
    let inner = size - 2;
    let fill = size - 4;
    
    let mut arcs = Vec::with_capacity(4);
    let mut gcs = Vec::with_capacity(4);
    
    if gc_lowbackground > 0 {
        arcs.push(Arc { 
            x: startx, y: starty, 
            width: size as u16, height: size as u16, 
            angle1: ANGLE45, angle2: ANGLE180 
        });
        gcs.push(gc_lowbackground);
        
        arcs.push(Arc { 
            x: startx + 1, y: starty + 1, 
            width: inner as u16, height: inner as u16, 
            angle1: ANGLE225, angle2: ANGLE180 
        });
        gcs.push(gc_lowbackground);
    }
    
    if gc_highlight > 0 {
        arcs.push(Arc { 
            x: startx, y: starty, 
            width: size as u16, height: size as u16, 
            angle1: ANGLE225, angle2: ANGLE180 
        });
        gcs.push(gc_highlight);
    }
    
    if gc_lowlight > 0 {
        arcs.push(Arc { 
            x: startx + 1, y: starty + 1, 
            width: inner as u16, height: inner as u16, 
            angle1: ANGLE45, angle2: ANGLE180 
        });
        gcs.push(gc_lowlight);
    }
    
    if !arcs.is_empty() {
        let mut groups: std::collections::HashMap<u32, Vec<Arc>> = std::collections::HashMap::new();
        
        for (arc, gc) in arcs.into_iter().zip(gcs.into_iter()) {
            groups.entry(gc).or_insert_with(Vec::new).push(arc);
        }
        
        for (gc, arclist) in groups {
            xconnection.poly_arc(window, gc, &arclist)?;
        }
    }
    
    //Center
    xconnection.poly_fill_arc(window, gc_highlight, &[Arc {x: startx + 2, y: starty + 2, width: fill as u16, height: fill as u16, angle1: 0, angle2: ANGLE360}])?;
    
    Ok(())
}

pub fn drawcheckbox<C: Connection>(xconnection: &C, window: u32, startx: i16, starty: i16, size: i16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32) -> Result<(), Box<dyn Error>> {
    
    let inner = size - 2;
    let fill = size - 4;
    
    if gc_lowbackground > 0 {
        xconnection.poly_fill_rectangle(window, gc_lowbackground, &[Rectangle {
            x: startx,
            y: starty,
            width: size as u16,
            height: size as u16,
        }])?;
    }
    
    let mut segments = Vec::new();
    let mut gcs = Vec::new();
    
    if gc_lowbackground > 0 {
        //Outer Top
        segments.push(Segment {
            x1: startx,
            y1: starty,
            x2: startx + size - 1,
            y2: starty,
        });
        gcs.push(gc_lowbackground);
        
        //Outer Left
        segments.push(Segment {
            x1: startx,
            y1: starty,
            x2: startx,
            y2: starty + size - 1,
        });
        gcs.push(gc_lowbackground);
    }
    
    if gc_lowlight > 0 {
        //Inner Top
        segments.push(Segment {
            x1: startx + 1,
            y1: starty + 1,
            x2: startx + size - 2,
            y2: starty + 1,
        });
        gcs.push(gc_lowlight);
        
        //Inner Left
        segments.push(Segment {
            x1: startx + 1,
            y1: starty + 1,
            x2: startx + 1,
            y2: starty + size - 2,
        });
        gcs.push(gc_lowlight);
    }
    
    if gc_highlight > 0 {
        //Outer Bottom
        segments.push(Segment {
            x1: startx,
            y1: starty + size - 1,
            x2: startx + size - 1,
            y2: starty + size - 1,
        });
        gcs.push(gc_highlight);
        
        //Outer Right
        segments.push(Segment {
            x1: startx + size - 1,
            y1: starty,
            x2: startx + size - 1,
            y2: starty + size - 1,
        });
        gcs.push(gc_highlight);
        
        //Inner Bottom
        segments.push(Segment {
            x1: startx + 1,
            y1: starty + size - 2,
            x2: startx + size - 2,
            y2: starty + size - 2,
        });
        gcs.push(gc_highbackground);
        
        //Inner Right
        segments.push(Segment {
            x1: startx + size - 2,
            y1: starty + 1,
            x2: startx + size - 2,
            y2: starty + size - 2,
        });
        gcs.push(gc_highbackground);
    }
    
    if !segments.is_empty() {
        let mut groups: std::collections::HashMap<u32, Vec<Segment>> = std::collections::HashMap::new();
        
        for (segment, gc) in segments.into_iter().zip(gcs.into_iter()) {
            groups.entry(gc).or_insert_with(Vec::new).push(segment);
        }
        
        for (gc, seglist) in groups {
            xconnection.poly_segment(window, gc, &seglist)?;
        }
    }
    
    if gc_highlight > 0 && fill > 0 {
        xconnection.poly_fill_rectangle(window, gc_highlight, &[Rectangle {
            x: startx + 2,
            y: starty + 2,
            width: fill as u16,
            height: fill as u16,
        }])?;
    }
    
    Ok(())
}

fn drawpnginternal<C: Connection>(xconnection: &C, window: u32, filename: &str, x: i16, y: i16, width: u16, height: u16, colour: Option<u32>, scale_mode: u8) -> Result<(), Box<dyn Error>> {
    let file = match File::open(filename) {
        Ok(f) => f,
        Err(e) => {
            // Colour square if we can't load the png.
            eprintln!("Can't Load Image: '{}': {}", filename, e);
            let gc = xconnection.generate_id()?;
            xconnection.create_gc(gc, window, &CreateGCAux::default().background(colour))?;
            xconnection.poly_fill_rectangle(window, gc, &[Rectangle {x, y, width, height}])?;
            xconnection.free_gc(gc)?;
            return Ok(());
        }
    };
    
    let decoder = Decoder::new(BufReader::new(file));
    let mut reader = decoder.read_info()?;
    let srcwidth = reader.info().width as usize;
    let srcheight = reader.info().height as usize;
    let colortype = reader.info().color_type;
    let mut buffer = vec![0; srcwidth * srcheight * 4];
    reader.next_frame(&mut buffer)?;
    
    let pixmap = xconnection.generate_id()?;
    xconnection.create_pixmap(24, pixmap, window, width, height)?;
    let gcimage = xconnection.generate_id()?;
    xconnection.create_gc(gcimage, pixmap, &CreateGCAux::default().background(colour))?;
    
    let bgcolour = colour.unwrap_or(0x00000000);
    let bg = [
        (bgcolour & 0xFF) as u8,
        ((bgcolour >> 8) & 0xFF) as u8, 
        ((bgcolour >> 16) & 0xFF) as u8,
        ((bgcolour >> 24) & 0xFF) as u8,
    ];
    
    let targetwidth = width as usize;
    let targetheight = height as usize;
    let pixelcount = targetwidth * targetheight;
    let mut imagedata = Vec::with_capacity(pixelcount * 4);
    
    if scale_mode == 0 {
        // Fast path: No scaling - direct pixel copy with clipping
        if colortype == png::ColorType::Rgba {
            for targety in 0..targetheight {
                for targetx in 0..targetwidth {
                    if targetx < srcwidth && targety < srcheight {
                        let idx = (targety * srcwidth + targetx) * 4;
                        let r = buffer[idx];
                        let g = buffer[idx + 1];
                        let b = buffer[idx + 2];
                        let a = buffer[idx + 3];
                        
                        match a {
                            0 => imagedata.extend_from_slice(&bg),
                            255 => imagedata.extend_from_slice(&[b, g, r, 255]),
                            _ => {
                                let alphaf = a as f32 / 255.0;
                                let invalpha = 1.0 - alphaf;
                                let blendedb = ((b as f32 * alphaf) + (bg[0] as f32 * invalpha)) as u8;
                                let blendedg = ((g as f32 * alphaf) + (bg[1] as f32 * invalpha)) as u8;
                                let blendedr = ((r as f32 * alphaf) + (bg[2] as f32 * invalpha)) as u8;
                                imagedata.extend_from_slice(&[blendedb, blendedg, blendedr, 255]);
                            }
                        }
                    } else {
                        imagedata.extend_from_slice(&bg);
                    }
                }
            }
        } else if colortype == png::ColorType::Rgb {
            for targety in 0..targetheight {
                for targetx in 0..targetwidth {
                    if targetx < srcwidth && targety < srcheight {
                        let idx = (targety * srcwidth + targetx) * 3;
                        let r = buffer[idx];
                        let g = buffer[idx + 1];
                        let b = buffer[idx + 2];
                        imagedata.extend_from_slice(&[b, g, r, 255]);
                    } else {
                        imagedata.extend_from_slice(&bg);
                    }
                }
            }
        } else {
            println!("Unsupported: {:?}", colortype);
            return Ok(());
        }
    } else {
        // Cover scaling with aspect ratio preservation
        let srcaspect = srcwidth as f32 / srcheight as f32;
        let targetaspect = targetwidth as f32 / targetheight as f32;
        
        let (scale, offsetx, offsety) = if srcaspect > targetaspect {
            let scale = targetheight as f32 / srcheight as f32;
            let scaledwidth = (srcwidth as f32 * scale) as usize;
            let offsetx = (scaledwidth.saturating_sub(targetwidth)) / 2;
            (scale, offsetx, 0)
        } else {
            let scale = targetwidth as f32 / srcwidth as f32;
            let scaledheight = (srcheight as f32 * scale) as usize;
            let offsety = (scaledheight.saturating_sub(targetheight)) / 2;
            (scale, 0, offsety)
        };
        
        let getsrcpixel = |srcx: usize, srcy: usize| -> (u8, u8, u8, u8) {
            if srcx >= srcwidth || srcy >= srcheight {
                return (0, 0, 0, 0);
            }
            let idx = (srcy * srcwidth + srcx) * 4;
            if colortype == png::ColorType::Rgba {
                (buffer[idx], buffer[idx + 1], buffer[idx + 2], buffer[idx + 3])
            } else if colortype == png::ColorType::Rgb {
                let rgbidx = (srcy * srcwidth + srcx) * 3;
                (buffer[rgbidx], buffer[rgbidx + 1], buffer[rgbidx + 2], 255)
            } else {
                (0, 0, 0, 0)
            }
        };
        
        for targety in 0..targetheight {
            for targetx in 0..targetwidth {
                let srcx = (((targetx + offsetx) as f32) / scale) as usize;
                let srcy = (((targety + offsety) as f32) / scale) as usize;
                
                let (r, g, b, a) = getsrcpixel(srcx, srcy);
                
                match a {
                    0 => imagedata.extend_from_slice(&bg),
                    255 => imagedata.extend_from_slice(&[b, g, r, 255]),
                    _ => {
                        let alphaf = a as f32 / 255.0;
                        let invalpha = 1.0 - alphaf;
                        let blendedb = ((b as f32 * alphaf) + (bg[0] as f32 * invalpha)) as u8;
                        let blendedg = ((g as f32 * alphaf) + (bg[1] as f32 * invalpha)) as u8;
                        let blendedr = ((r as f32 * alphaf) + (bg[2] as f32 * invalpha)) as u8;
                        imagedata.extend_from_slice(&[blendedb, blendedg, blendedr, 255]);
                    }
                }
            }
        }
    }
    
    xconnection.put_image(ImageFormat::Z_PIXMAP, pixmap, gcimage, width, height, 0, 0, 0, 24, &imagedata)?;
    xconnection.copy_area(pixmap, window, gcimage, 0, 0, x, y, width, height)?;
    xconnection.free_pixmap(pixmap)?;
    xconnection.free_gc(gcimage)?;
    xconnection.flush()?;
    Ok(())
}

pub fn drawpng<C: Connection>(xconnection: &C, window: u32, filename: &str, x: i16, y: i16, width: u16, height: u16, colour: Option<u32>) -> Result<(), Box<dyn Error>> {
    // I only want to support one image type. I didn't want to use png, but all icon packs use png so it'll probably stay.
    // Vector would be fun, but probably too resource intensive.
    // Fast path: no scaling for embedded performance
    drawpnginternal(xconnection, window, filename, x, y, width, height, colour, 0)
}

pub fn drawpngcover<C: Connection>(xconnection: &C, window: u32, filename: &str, x: i16, y: i16, width: u16, height: u16, colour: Option<u32>) -> Result<(), Box<dyn Error>> {
    drawpnginternal(xconnection, window, filename, x, y, width, height, colour, 1)
}