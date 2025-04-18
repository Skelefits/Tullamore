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
use x11rb::wrapper::ConnectionExt as _;
use x11rb::COPY_DEPTH_FROM_PARENT;


const HIGHBACKGROUND_COLOUR: usize = 0;
const LOWBACKGROUND_COLOUR: usize = 1;
const HIGHLIGHT_COLOUR: usize = 2;
const LOWLIGHT_COLOUR: usize = 3;
const WALLPAPER_COLOUR: usize = 4;
const TITLEBAR_COLOUR: usize = 5;

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
		0x0000A8, //TITLEBAR_COLOUR
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
    //let handle = thread::spawn(|| { //async this maybe, or remove
        if let Err(e) = desktop() {
            eprintln!("Error in panel: {}", e);
        }
    //});
    //handle.join().unwrap();

    Ok(())
}

fn desktop() -> Result<(), Box<dyn Error>> {
	//let width = 640 as i16;
	//let height = 480 as i16;
	

	
	
	
    let (xconnection, screenid) = x11rb::connect(Some(":0"))?;
    let screen = &xconnection.setup().roots[screenid];
	
	let width = screen.width_in_pixels as i16;
	let height = screen.height_in_pixels as i16;
	
    //let window = xconnection.generate_id()?; 
	let window = screen.root;
    xconnection.create_window(0, window, screen.root, 0, 0, width as u16, height as u16, 0, WindowClass::INPUT_OUTPUT, screen.root_visual, &CreateWindowAux::default().background_pixel(COLOURS[WALLPAPER_COLOUR]),)?;
	let panel = xconnection.generate_id()?;
	
	let panelheight = 28;
	
	
	xconnection.create_window(0, panel, window, 0, height - panelheight as i16, width as u16, panelheight, 0, WindowClass::INPUT_OUTPUT, screen.root_visual, &CreateWindowAux::default(),)?;
	
	xconnection.change_window_attributes(window, &ChangeWindowAttributesAux::default().event_mask(EventMask::BUTTON_PRESS | EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY).background_pixel(COLOURS[WALLPAPER_COLOUR]).override_redirect(1),)?;

    //Graphic contexts...
    let gc_highbackground = xconnection.generate_id()?;
	let gc_lowbackground = xconnection.generate_id()?;
    let gc_highlight = xconnection.generate_id()?;
	let gc_lowlight = xconnection.generate_id()?;	
	let gc_titlebar = xconnection.generate_id()?;	
	let gc_titlebartext = xconnection.generate_id()?;

    //Show window.
    xconnection.map_window(panel)?;
	
	
	
	xconnection.create_gc(gc_lowlight, window, &CreateGCAux::default().foreground(COLOURS[LOWLIGHT_COLOUR]).background(COLOURS[HIGHBACKGROUND_COLOUR]))?;
	xconnection.create_gc(gc_highbackground, window, &CreateGCAux::default().foreground(COLOURS[HIGHBACKGROUND_COLOUR]))?;
	xconnection.create_gc(gc_lowbackground, window, &CreateGCAux::default().foreground(COLOURS[LOWBACKGROUND_COLOUR]))?;
	xconnection.create_gc(gc_highlight, window, &CreateGCAux::default().foreground(COLOURS[HIGHLIGHT_COLOUR]))?;
	xconnection.create_gc(gc_titlebar, window, &CreateGCAux::default().foreground(COLOURS[TITLEBAR_COLOUR]).background(COLOURS[HIGHBACKGROUND_COLOUR]))?;
	xconnection.create_gc(gc_titlebartext, window, &CreateGCAux::default().foreground(COLOURS[HIGHLIGHT_COLOUR]).background(COLOURS[TITLEBAR_COLOUR]))?;

    xconnection.poly_fill_rectangle(panel, gc_highbackground, &[Rectangle { x: 0, y: 0, width: width as u16, height: panelheight }])?; //Draw panel background.
    xconnection.poly_line(CoordMode::PREVIOUS, panel, gc_highlight, &[Point { x: 0, y: 1 }, Point { x: width as i16, y: 0 }])?; //Draw panel highlight.

	//Draw notification box.
	
	//calculate the size of the notification box.
	let icons = 3;
	let notification = (icons*20) + 60;
	
	drawdepressedframe(&xconnection, panel, width - 3, 4, notification, 21, gc_highlight, gc_lowbackground)?;

	let clockheight = 20;
	let (mut epoch, mut pminute, mut phour) = drawclock(&xconnection, panel, gc_lowlight, width, clockheight)?;


	
	//Load notification graphic. 
    drawpng(&xconnection, panel, "audio-volume-muted.png", width - notification, 7, 16, 16, COLOURS[HIGHBACKGROUND_COLOUR])?;
	drawpng(&xconnection, panel, "network-offline.png", width - notification + 20, 7, 16, 16, COLOURS[HIGHBACKGROUND_COLOUR])?;
	drawpng(&xconnection, panel, "weather-snow.png", width - notification + 40, 7, 16, 16, COLOURS[HIGHBACKGROUND_COLOUR])?;


	//Put panel on top.
	xconnection.configure_window(panel, &ConfigureWindowAux::default().stack_mode(StackMode::ABOVE))?;

    xconnection.flush()?;




    let border = 4 as u16;
    let titlebar = 18 as u16;


    let test1 = xconnection.generate_id()?;
    xconnection.create_window(COPY_DEPTH_FROM_PARENT, test1, screen.root, 100, 100, 300, 200, 0, WindowClass::INPUT_OUTPUT, 0, &CreateWindowAux::new().background_pixel(screen.white_pixel).event_mask(EventMask::EXPOSURE | EventMask::BUTTON_PRESS),)?;
    xconnection.change_property8(PropMode::REPLACE, test1, AtomEnum::WM_NAME, AtomEnum::STRING, b"First Window")?;
	//second window test
    let test2 = xconnection.generate_id()?;
    xconnection.create_window(COPY_DEPTH_FROM_PARENT, test2, screen.root, 500, 500, 300, 200, 0, WindowClass::INPUT_OUTPUT, 0, &CreateWindowAux::new().background_pixel(screen.white_pixel).event_mask(EventMask::EXPOSURE | EventMask::BUTTON_PRESS),)?;
    xconnection.change_property8(PropMode::REPLACE, test2, AtomEnum::WM_NAME, AtomEnum::STRING, b"Second Window")?;


	createborder(&xconnection, &screen, test1, width, height, border, titlebar, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext,)?;
	createborder(&xconnection, &screen, test2, width, height, border, titlebar, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext,)?;

    xconnection.flush()?;





	let mut moving: Option<u32> = None;
	let mut drag: Option<(i16, i16)> = None;
	let mut origin: Option<(i16, i16)> = None;


    loop {
		
		let epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
		let cminute = ((epoch / 60) % 60) as u8;
		updateclock(&xconnection, panel, gc_lowlight, phour, pminute, cminute, width, clockheight);
		
        let event = xconnection.wait_for_event()?;
            match event {
				
				
				//stuff for wm
				Event::MapRequest(target) => {
					println!("MapRequest Target: {:?}", target.window);
					//Add frame and title.
					createborder(&xconnection, &screen, target.window, width, height, border, titlebar, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar,gc_titlebartext,)?;
				}



				
				
				
				
				Event::ConfigureRequest(target) => {
					let aux = ConfigureWindowAux::from_configure_request(&target);
					xconnection.configure_window(target.window, &aux)?;
					xconnection.flush()?;
				}
				Event::DestroyNotify(destroy) => {

				}
				//For moving windows around!
				Event::MotionNotify(motion) => {
					if let (Some(win), Some((start_x, start_y)), Some((win_x, win_y))) =
						(moving, drag, origin)
					{
						let dx = motion.root_x - start_x;
						let dy = motion.root_y - start_y;
						let new_x = win_x + dx;
						let new_y = win_y + dy;

						xconnection.configure_window(
							win,
							&ConfigureWindowAux::new().x(new_x as i32).y(new_y as i32),
						)?;
					}
				}
				
				
				
				
				
				
				//For releasing the window! Redraw the frame!
				Event::ButtonRelease(_) => {
					if let Ok(root_tree) = xconnection.query_tree(screen.root)?.reply() {
						for &target in &root_tree.children {
							//Skip the panel window
							if target != panel {
								if let Ok(tree) = xconnection.query_tree(target)?.reply() {
									//If this window has children, it is likely a frame.
									if !tree.children.is_empty() {
										if let Ok(geom) = xconnection.get_geometry(target)?.reply() {
											let width = geom.width as i16;
											let height = geom.height as i16;
											//Redraw frame.
											updateborder(&xconnection, target, tree.children.last().copied().unwrap_or(target), width, height, titlebar, border, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext)?;
										}
									}
								}
							}
						}
					}
					xconnection.flush()?;
					
					moving = None;
					drag = None;
					origin = None;
					
				}
				
				//We need a lot more comments here!
                Event::ButtonPress(press) => {
					
					
					if press.detail == 1 { //Left mouse button pressed.
						let target = press.event;
						if let Ok(tree) = xconnection.query_tree(target)?.reply() {
							//Focus window. Layer it just under the panel.
							xconnection.configure_window(target, &ConfigureWindowAux::default().sibling(panel).stack_mode(StackMode::BELOW))?;
							let aframe = !tree.children.is_empty();
							//This window has a child (is a frame) and is licked in the titlebar.
							if aframe && press.event_y < titlebar as i16 {
								//Start dragging the window.
								moving = Some(target);
								drag = Some((press.root_x, press.root_y));
								
								if let Ok(geom) = xconnection.get_geometry(target)?.reply() {
									origin = Some((geom.x as i16, geom.y as i16));
								}
							} else {
								//Do we have a frame?
								if tree.parent != 0 {
									//Focus parent (frame) if there is one.
									xconnection.configure_window(tree.parent, &ConfigureWindowAux::default().sibling(panel).stack_mode(StackMode::BELOW))?;
								}
							}
						}
					
					
					
					
					
					
					
					
						if press.event_y > height - 20 && press.event_y < height - 5 {
							if press.event_x > width - notification {
								if press.event_x < width - 60 {
									let clicked_icon = icons - 1 - (press.event_x - (width - notification)) / 20;
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
							println!("Mouse button pressed at ({}, {}) with button: {}", press.event_x, press.event_y, press.detail);
						}
					}
                }
                Event::Error(_) => println!("bug bug"),
				_ => (),
            }
		xconnection.flush()?;
    }
    println!("wat?");
}

fn createborder(xconnection: &impl x11rb::connection::Connection, screen: &x11rb::protocol::xproto::Screen, target: u32, width: i16, height: i16, border: u16, titlebar: u16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32, gc_titlebar: u32, gc_titlebartext: u32) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(geom) = xconnection.get_geometry(target)?.reply() {

		//Calculate frame's dimensions.
		let fwidth = geom.width + border + border;
		let fheight = geom.height + titlebar + border + border;

		//Calculate frame's origin.
		let fx = (geom.x - border as i16).max(0).min(width - fwidth as i16);
		let fy = (geom.y - (titlebar - border) as i16).max(0).min(height - fheight as i16);

		//Create frame and put the target into into it.
		let frame = xconnection.generate_id()?;
		xconnection.create_window( COPY_DEPTH_FROM_PARENT, frame, screen.root, fx, fy, fwidth, fheight, 0, WindowClass::INPUT_OUTPUT, 0, &CreateWindowAux::new().background_pixel(COLOURS[HIGHBACKGROUND_COLOUR]).event_mask(EventMask::EXPOSURE | EventMask::BUTTON_PRESS | EventMask::POINTER_MOTION | EventMask::BUTTON_RELEASE),)?;
		//Set the target's frame to 0, in case it has one for some reason.
		xconnection.configure_window(target, &ConfigureWindowAux::new().border_width(0))?;

		xconnection.reparent_window(target, frame, border as i16, (border + titlebar) as i16,)?;
		xconnection.map_window(frame)?;
		xconnection.map_window(target)?;

        updateborder(xconnection, frame, target, fwidth as i16, fheight as i16, titlebar, border, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext, )?;

		xconnection.flush()?;
	}
    Ok(())
}

fn updateborder<C: x11rb::connection::Connection>(xconnection: &C, frame: u32, target: u32, width: i16, height: i16, titlebar: u16, border: u16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32, gc_titlebar: u32, gc_titlebartext: u32) -> Result<(), Box<dyn std::error::Error>> {
    windowborder(xconnection, frame, width, height, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground)?;
    //if height > (titlebar + 2 * border) as i16 {
        drawtitlebar(xconnection, frame, width - 8, titlebar as i16, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar)?;
        drawtitletext(xconnection, frame, gc_titlebartext, target, 8, (titlebar as i16) - 1)?;
    //}
    Ok(())
}

fn grabwindowtitle<C: x11rb::connection::Connection>(xconnection: &C, window: u32,) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let reply = xconnection.get_property(false, window, AtomEnum::WM_NAME, AtomEnum::STRING, 0, 1024)?.reply()?;
    if reply.value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(String::from_utf8_lossy(&reply.value).to_string()))
    }
}

fn drawtitletext<C: x11rb::connection::Connection>(xconnection: &C, drawable: u32, gc: u32, window: u32, x: i16, y: i16,) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(title) = grabwindowtitle(xconnection, window)? {
        xconnection.image_text8(drawable, gc, x, y, title.as_bytes())?;
    }
    Ok(())
}

fn windowborder<C: Connection>(xconnection: &C, window: u32, width: i16, height: i16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32) -> Result<(), Box<dyn Error>> {
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




fn updateclock<C: Connection>(xconnection: &C, window: u32, gc: u32, mut phour: u8, pminute: u8, cminute: u8, width: i16, height: i16) -> Result<(u8, u8), Box<dyn Error>> {
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



fn drawtitlebar<C: Connection>(xconnection: &C, window: u32, width: i16, height: i16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32, gc_titlebar: u32) -> Result<(), Box<dyn Error>> {

	
	xconnection.poly_fill_rectangle(window, gc_titlebar, &[Rectangle {x: 4, y: 4, width: width as u16, height: height as u16,}])?;
	
	drawbumpyframe(&xconnection, window, width - 14, 6, 15, 13, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground)?;
	drawbumpyframe(&xconnection, window, width - 30, 6, 15, 13, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground)?;
	drawbumpyframe(&xconnection, window, width - 46, 6, 15, 13, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground)?;
	
    Ok(())
}

				

fn drawbumpyframe<C: Connection>(xconnection: &C, window: u32, startx: i16, starty: i16, framewidth: i16, frameheight: i16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32) -> Result<(), Box<dyn Error>> {
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
