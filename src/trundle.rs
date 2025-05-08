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
	println!("{}", width - 14);
	
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

	drawbumpyframe(&xconnection, window, 2, 4, 54, 21, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground)?;
	drawpng(&xconnection, window, "computer.png", 6, 7, 16, 16, COLOURS[HIGHBACKGROUND_COLOUR])?;
	
	xconnection.image_text8(window, gc_lowlight, 24, 19, "S".as_bytes());
	xconnection.image_text8(window, gc_lowlight, 24+5, 19, "t".as_bytes());
	xconnection.image_text8(window, gc_lowlight, 24+5+5, 19, "ar".as_bytes());
	xconnection.image_text8(window, gc_lowlight, 24+5+5+11, 19, "t".as_bytes());
    Ok(())
}

pub fn drawdepressedbumpyframe<C: Connection>(xconnection: &C, window: u32, startx: i16, starty: i16, framewidth: i16, frameheight: i16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32, gc_highcheckers: u32) -> Result<(), Box<dyn Error>> {
	drawbumpyframe(&xconnection, window, startx, starty, framewidth, frameheight, gc_lowlight, gc_highlight, gc_highbackground, gc_highbackground)?;
	
	xconnection.poly_line(CoordMode::PREVIOUS, window, gc_lowbackground, &[
		Point { x: startx + 1, y: starty + frameheight - 2 },
		Point { x: 0, y: 3 - frameheight },
		Point { x: framewidth - 3, y: 0 },
	])?;
	
	xconnection.poly_line(CoordMode::PREVIOUS, window, gc_highlight, &[
		Point { x: startx + 2, y: 6 },
		Point { x: framewidth - 4, y: 0 },
	])?;
	
	xconnection.poly_fill_rectangle(window, gc_highcheckers, &[Rectangle { x: startx + 2, y: starty + 3, width: framewidth as u16 - 3, height: frameheight as u16 - 4}])?;
	
	
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
	xconnection.poly_fill_rectangle(window, gc_highbackground, &[Rectangle {x: startx + 1, y: starty + 1, width: (framewidth as u16) - 2, height: (frameheight as u16) - 2,}])?;
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

pub fn drawpng<C: Connection>(xconnection: &C, window: u32, filename: &str, x: i16, y: i16, height: u16, width: u16, colour: Option<u32>) -> Result<(), Box<dyn Error>> {
	//I only want to support one image type. I didn't want to use png, but all icon packs use png so it'll probably stay.
	//Vector would be fun, but probably too resource intensive.
    let file = File::open(filename)?;
    let decoder = Decoder::new(BufReader::new(file));
    let mut reader = decoder.read_info()?;

    let mut buffer = vec![0; (height * width * 4) as usize];
    reader.next_frame(&mut buffer)?;

    let pixmap = xconnection.generate_id()?;
    xconnection.create_pixmap(24, pixmap, window, width, height)?;

    let gc_image = xconnection.generate_id()?;
    xconnection.create_gc(gc_image, pixmap, &CreateGCAux::default().background(colour))?;

    let bg_color = colour.unwrap_or(0x00000000);
    let bg_r = ((bg_color >> 16) & 0xFF) as u8;
    let bg_g = ((bg_color >> 8) & 0xFF) as u8;
    let bg_b = (bg_color & 0xFF) as u8;
    let bg_a = ((bg_color >> 24) & 0xFF) as u8;

    let mut image_data = Vec::with_capacity((width as usize) * (height as usize) * 4);

    let info = &reader.info();

    if info.color_type == png::ColorType::Rgba {
        //alpha channel
        for chunk in buffer.chunks(4) {
            let r = chunk[0];
            let g = chunk[1];
            let b = chunk[2];
            let a = chunk[3];

            if a == 0 {
                //replace alpha with background colour
                image_data.push(bg_b);
                image_data.push(bg_g);
                image_data.push(bg_r);
                image_data.push(bg_a);
            } else {
                //dont replace, some icons are looking weird, may have to do more adjustment
                image_data.push(b);
                image_data.push(g);
                image_data.push(r);
                image_data.push(a);
            }
        }
    } else if info.color_type == png::ColorType::Rgb {
        //no alpha channel, is this ever going to be called? need to test or research icon packs
        for chunk in buffer.chunks(3) {
            let r = chunk[0];
            let g = chunk[1];
            let b = chunk[2];

            image_data.push(b);
            image_data.push(g);
            image_data.push(r);
            image_data.push(255);
        }
    } else {
        //format not found
        println!("Unsupported: {:?}", info.color_type);
        return Ok(());
    }

    xconnection.put_image(ImageFormat::Z_PIXMAP, pixmap, gc_image, width, height, 0, 0, 0, 24, &image_data)?;

    xconnection.copy_area( pixmap, window, gc_image, 0, 0, x, y, width, height,)?;

    xconnection.free_pixmap(pixmap)?;
    xconnection.free_gc(gc_image)?;
    xconnection.flush()?;

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
    
    println!("{}:{}", phour, pminute);

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