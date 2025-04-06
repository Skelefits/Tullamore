use lazy_static::lazy_static;
use png::Decoder;
use std::error::Error;
use std::thread;
use std::fs::File;
use std::io::{self, Read, BufReader};
use std::time::{SystemTime, Duration};
use x11rb::connection::{Connection};
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::errors::ConnectionError;

const HIGHBACKGROUND_COLOUR: usize = 0;
const LOWBACKGROUND_COLOUR: usize = 1;
const HIGHLIGHT_COLOUR: usize = 2;
const LOWLIGHT_COLOUR: usize = 3;
const WALLPAPER_COLOUR: usize = 4;

//struct Element {
//	command: Vec<(u8, u8, u8)>, //index, command, colour
//    coordinates: Vec<(u16, u16)>,
//}

lazy_static! {
	static ref COLOURS: Vec<Option<u32>> = loadcolours("colours.txt", [
        0xBBBBBB, //HIGHBACKGROUND_COLOUR
        0x888888, //LOWBACKGROUND_COLOUR
        0xFFFFFF, //HIGHLIGHT_COLOUR
        0x000000, //LOWLIGHT_COLOUR
        0x008080, //WALLPAPER_COLOUR
    ]);
	
    //let mut depressedborder = Element {
    //    command: vec![(1, 255, 0), (2, 128, 1)],
    //    coordinates: vec![(0, 21), (30, 40)],
    //};
	
}

fn loadcolours<const N: usize>(file_path: &str, default: [u32; N]) -> Vec<Option<u32>> {
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

fn main() -> Result<(), Box<dyn Error>> {
    let handle = thread::spawn(|| { //async this maybe, or remove
        if let Err(e) = panel() {
            eprintln!("Error in panel: {}", e);
        }
    });
    handle.join().unwrap();

    Ok(())
}

fn panel() -> Result<(), Box<dyn Error>> {
	let width = 640 as i16;
	let height = 480 as i16;
	
    let (xconnection, screenid) = x11rb::connect(Some(":0"))?; // Specify display explicitly
    let screen = &xconnection.setup().roots[screenid];
    let window = xconnection.generate_id()?; 
    xconnection.create_window(0, window, screen.root, 100, 100, width as u16, height as u16, 0, WindowClass::INPUT_OUTPUT, screen.root_visual, &CreateWindowAux::default(),)?;
	
	xconnection.change_window_attributes(window, &ChangeWindowAttributesAux::default().event_mask(EventMask::BUTTON_PRESS).background_pixel(COLOURS[WALLPAPER_COLOUR]))?;

    //Graphic contexts...
    let gc_highbackground = xconnection.generate_id()?;
	let gc_lowbackground = xconnection.generate_id()?;
    let gc_highlight = xconnection.generate_id()?;
	let gc_lowlight = xconnection.generate_id()?;	

    //Show window.
    xconnection.map_window(window)?;
	
	xconnection.create_gc(gc_lowlight, window, &CreateGCAux::default().foreground(COLOURS[LOWLIGHT_COLOUR]).background(COLOURS[HIGHBACKGROUND_COLOUR]))?;
	xconnection.create_gc(gc_highbackground, window, &CreateGCAux::default().foreground(COLOURS[HIGHBACKGROUND_COLOUR]))?;
	xconnection.create_gc(gc_lowbackground, window, &CreateGCAux::default().foreground(COLOURS[LOWBACKGROUND_COLOUR]))?;
	xconnection.create_gc(gc_highlight, window, &CreateGCAux::default().foreground(COLOURS[HIGHLIGHT_COLOUR]))?;

    xconnection.poly_fill_rectangle(window, gc_highbackground, &[Rectangle { x: 0, y: height - 28, width: width as u16, height: 28 }])?; //Draw panel background.
    xconnection.poly_line(CoordMode::PREVIOUS, window, gc_highlight, &[Point { x: 0, y: height - 27 }, Point { x: width as i16, y: 0 }])?; //Draw panel highlight.

	//Draw notification box.
	
	//calculate the size of the notification box.
	let icons = 3;
	let notification = (icons*20) + 60;
	
	drawdepressedframe(&xconnection, window, width - 3, height - 24, notification, 21, gc_highlight, gc_lowbackground)?;

	let clockheight = height - 8;
	let (mut epoch, mut pminute, mut phour) = drawclock(&xconnection, window, gc_lowlight, width, clockheight)?;


	
	//Load notification graphic. 
    drawpng(&xconnection, window, "audio-volume-muted.png", width - notification, height - 21, 16, 16, COLOURS[HIGHBACKGROUND_COLOUR])?;
	drawpng(&xconnection, window, "network-offline.png", width - notification + 20, height - 21, 16, 16, COLOURS[HIGHBACKGROUND_COLOUR])?;
	drawpng(&xconnection, window, "weather-snow.png", width - notification + 40, height - 21, 16, 16, COLOURS[HIGHBACKGROUND_COLOUR])?;


    xconnection.flush()?;

    loop {
		

		
		//let handle = thread::spawn(|| { //async this probably
			//Maybe put the buttonpress into its own thread instead?
			let epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
			let cminute = ((epoch / 60) % 60) as u8;
			updateclock(&xconnection, window, gc_lowlight, phour, pminute, cminute, width, clockheight);

		//});
		
        let event = xconnection.wait_for_event()?;
            match event {
				//We need a lot more comments here!
                Event::ButtonPress(button_event) => {
					if button_event.event_y > height - 20 && button_event.event_y < height - 5 {
						if button_event.event_x > width - notification {
							if button_event.event_x < width - 60 {
								let clicked_icon = icons - 1 - (button_event.event_x - (width - notification)) / 20;
								if clicked_icon >= 0 {
									println!("Icon index: {}", clicked_icon + 1);
								}
							} else {
								println!("Clock clicked!");
							}
						} else {
							println!("Middle of panel clicked!");
						}
					} else {
						println!("Mouse button pressed at ({}, {}) with button: {}", button_event.event_x, button_event.event_y, button_event.detail);
					}
                }
                Event::Error(_) => println!("bug bug"),
                _ => println!("Unknown event!"),
            }
		xconnection.flush()?;
    }
    println!("wat?");
}

fn updateclock<C: Connection>(xconnection: &C, window: u32, gc: u32, mut phour: u8, pminute: u8, cminute: u8, width: i16, height: i16) -> Result<(u8, u8), Box<dyn Error>> {
	println!("UpdateClock Called!");
	if cminute != pminute {
		println!("cminute != pminute");
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
		println!("tminute {}", tminute);
	}
    Ok((phour, cminute))
}

fn drawdepressedframe<C: Connection>(xconnection: &C, window: u32, startx: i16, starty: i16, framewidth: i16, frameheight: i16, gc_highlight: u32, gc_lowbackground: u32) -> Result<(), Box<dyn Error>> {
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


fn drawclock<C: Connection>(xconnection: &C, window: u32, gc_lowlight: u32, width: i16, clockheight: i16) -> Result<(u64, u8, u8), Box<dyn Error>> {
    //Clock separator.
    xconnection.poly_point(CoordMode::PREVIOUS, window, gc_lowlight, &[ 
        Point { x: width - 42, y: clockheight - 2 }, 
        Point { x: 0, y: -4 } 
    ])?;
    
    let epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    let mut pminute = ((epoch / 60) % 60) as u8;
    let mut phour = ((epoch / 3600) % 24) as u8;
    let mut chour = phour;
    
    // Convert to 12-hour format
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

fn drawpng<C: Connection>(xconnection: &C, window: u32, filename: &str, x: i16, y: i16, height: u16, width: u16, colour: Option<u32>) -> Result<(), Box<dyn Error>> {
	//I only want to support one image type. I didn't want to use png, but all icon packs use png so it'll probably stay.
	//Vector would be fun, but probably too resource intensive.
    let file = File::open(filename)?;
    let decoder = Decoder::new(BufReader::new(file));
    let mut reader = decoder.read_info()?;

    let mut buffer = vec![0; (height * width * 4) as usize];
    reader.next_frame(&mut buffer)?; // Load in image data.

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
