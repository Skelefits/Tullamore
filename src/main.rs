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

mod window;
use window::redrawframes;
use window::updateborder;
use window::createborder;
use window::drawpanelwindows;
mod trundle;
use trundle::windowborder;
use trundle::drawtitlebar;
use trundle::grabwindowtitle;
use trundle::drawtitletext;
use trundle::drawbumpyframe;
use trundle::drawdepressedbumpyframe;
use trundle::drawdepressedframe;
use trundle::drawstartbutton;
use trundle::drawpng;
use trundle::drawclock;
use trundle::updateclock;

use trundle::{
    COLOURS,
    HIGHBACKGROUND_COLOUR,
    LOWBACKGROUND_COLOUR,
    HIGHLIGHT_COLOUR,
    LOWLIGHT_COLOUR,
    WALLPAPER_COLOUR,
    TITLEBAR_COLOUR
};

struct WindowState {
    window: Window,
    frame: Window,
    title: String,
    x: i16,
    y: i16,
	z: u32,
    width: i16,
    height: i16,
    map: u8, //0 for hidden taskbar, 1 for hidden notification bar, 2 for visible and focused, 3 for visble and not focused
    workspace: u8,
    group: u8,
}

const FASTDRAG: bool = true;

//struct Element {
//	command: Vec<(u8, u8, u8)>, //index, command, colour
//    coordinates: Vec<(u16, u16)>,
//}





fn main() -> Result<(), Box<dyn Error>> {
    //let handle = thread::spawn(|| { //async this maybe, or remove
        if let Err(e) = desktop() {
            eprintln!("Error in panel: {}", e);
        }
    //});
    //handle.join().unwrap();

    Ok(())
}

fn drawxoroutline<C: Connection>(xconnection: &C, window: u32, gc: u32, x: i16, y: i16, width: u16, height: u16,) -> Result<(), Box<dyn Error>> {
    let points = [
        Point { x, y },
        Point { x: x + width as i16, y },
        Point { x: x + width as i16, y: y + height as i16 },
        Point { x, y: y + height as i16 },
        Point { x, y },
    ];
    xconnection.poly_line(CoordMode::ORIGIN, window, gc, &points)?;
    Ok(())
}

fn makepattern<C: Connection>(xconnection: &C, window: u32, firstcolour: u32, secondcolour: u32) -> Result<u32, Box<dyn Error>> {
    let pixmap = xconnection.generate_id()?;
    let gc = xconnection.generate_id()?;
    let temp_gc1 = xconnection.generate_id()?;
    let temp_gc2 = xconnection.generate_id()?;

    // Create 4x4 pixmap
    xconnection.create_pixmap(24, pixmap, window, 2, 2)?;

    // Create temporary GCs
    xconnection.create_gc(temp_gc1, pixmap, &CreateGCAux::default().foreground(firstcolour))?;
    xconnection.create_gc(temp_gc2, pixmap, &CreateGCAux::default().foreground(secondcolour))?;

    // Draw checker pattern
    xconnection.poly_fill_rectangle(pixmap, temp_gc1, &[
        Rectangle { x: 0, y: 0, width: 1, height: 1 },
        Rectangle { x: 1, y: 1, width: 1, height: 1 }
    ])?;
    xconnection.poly_fill_rectangle(pixmap, temp_gc2, &[
        Rectangle { x: 1, y: 0, width: 1, height: 1 },
        Rectangle { x: 0, y: 1, width: 1, height: 1 }
    ])?;

    // Create tiled GC
    xconnection.create_gc(gc, window, &CreateGCAux::default()
        .tile(pixmap)
        .fill_style(FillStyle::TILED)
    )?;

    // Cleanup
    xconnection.free_gc(temp_gc1)?;
    xconnection.free_gc(temp_gc2)?;
    xconnection.free_pixmap(pixmap)?;

    Ok(gc)
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
	let gc_xor = xconnection.generate_id()?;
	let gc_highcheckers = xconnection.generate_id()?;
	let gc_lowcheckers = xconnection.generate_id()?;

    //Show window.
    xconnection.map_window(panel)?;
	
	
	
	xconnection.create_gc(gc_lowlight, window, &CreateGCAux::default().foreground(COLOURS[LOWLIGHT_COLOUR]).background(COLOURS[HIGHBACKGROUND_COLOUR]))?;
	xconnection.create_gc(gc_highbackground, window, &CreateGCAux::default().foreground(COLOURS[HIGHBACKGROUND_COLOUR]))?;
	xconnection.create_gc(gc_lowbackground, window, &CreateGCAux::default().foreground(COLOURS[LOWBACKGROUND_COLOUR]))?;
	xconnection.create_gc(gc_highlight, window, &CreateGCAux::default().foreground(COLOURS[HIGHLIGHT_COLOUR]))?;
	xconnection.create_gc(gc_titlebar, window, &CreateGCAux::default().foreground(COLOURS[TITLEBAR_COLOUR]).background(COLOURS[HIGHBACKGROUND_COLOUR]))?;
	xconnection.create_gc(gc_titlebartext, window, &CreateGCAux::default().foreground(COLOURS[HIGHLIGHT_COLOUR]).background(COLOURS[TITLEBAR_COLOUR]))?;
	xconnection.create_gc(gc_xor, window, &CreateGCAux::default().function(Some(GX::XOR)).foreground(0xFFFFFF).subwindow_mode(SubwindowMode::INCLUDE_INFERIORS),)?;


	// Create checker pattern GCs
	let gc_highcheckers = makepattern(&xconnection, window, COLOURS[HIGHLIGHT_COLOUR].unwrap(), COLOURS[HIGHBACKGROUND_COLOUR].unwrap())?;
	let gc_lowcheckers = makepattern(&xconnection, window, COLOURS[LOWLIGHT_COLOUR].unwrap(), COLOURS[LOWBACKGROUND_COLOUR].unwrap())?;


	

    xconnection.poly_fill_rectangle(panel, gc_highbackground, &[Rectangle { x: 0, y: 0, width: width as u16, height: panelheight }])?; //Draw panel background.
    xconnection.poly_line(CoordMode::PREVIOUS, panel, gc_highlight, &[Point { x: 0, y: 1 }, Point { x: width as i16, y: 0 }])?; //Draw panel highlight.

	//Draw notification box.
	
	//calculate the size of the notification box.
	let icons = 3;
	let notification = (icons*20) + 60;
	
	//Notification - Depression
	drawdepressedframe(&xconnection, panel, width - 3, 4, notification, 21, gc_highlight, gc_lowbackground)?;

	//Start Bump
	drawstartbutton(&xconnection, panel, 2, 4, 54, 21, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground)?;



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
	xconnection.change_window_attributes(test1, &ChangeWindowAttributesAux::default().override_redirect(0))?;
	xconnection.map_window(test1)?;
	createborder(&xconnection, &screen, test1, width, height, border, titlebar, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext,)?;
	
	
	//second window test
    let test2 = xconnection.generate_id()?;
    xconnection.create_window(COPY_DEPTH_FROM_PARENT, test2, screen.root, 200, 200, 300, 200, 0, WindowClass::INPUT_OUTPUT, 0, &CreateWindowAux::new().background_pixel(screen.white_pixel).event_mask(EventMask::EXPOSURE | EventMask::BUTTON_PRESS),)?;
    xconnection.change_property8(PropMode::REPLACE, test2, AtomEnum::WM_NAME, AtomEnum::STRING, b"Second Window")?;
	xconnection.change_window_attributes(test2, &ChangeWindowAttributesAux::default().override_redirect(0))?;
	xconnection.map_window(test2)?;
	createborder(&xconnection, &screen, test2, width, height, border, titlebar, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext,)?;

	
	


	//Draw window boxes on the panel.
	drawpanelwindows(&xconnection, panel, 61, width - notification - 67, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_highcheckers)?;




    xconnection.flush()?;





	let mut moving: Option<u32> = None;
	let mut drag: Option<(i16, i16)> = None;
	let mut origin: Option<(i16, i16)> = None;

	let mut dragpressoffsetx = 0;
	let mut dragpressoffsety = 0;

	let mut xordrawn: Option<(i16, i16, u16, u16)> = None;

    loop {
		//Lets collect everything we have to do and execute at the end.

		
		
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
					if let (Some(win), Some((start_x, start_y)), Some((win_x, win_y))) = (moving, drag, origin) {
						let dx = motion.root_x - start_x;
						let dy = motion.root_y - start_y;
						let new_x = win_x + dx;
						let new_y = win_y + dy;

					if FASTDRAG {
						if let Some((lx, ly, lw, lh)) = xordrawn {
							//Draw XOR Outline to overwrite previous one.
							drawxoroutline(&xconnection, screen.root, gc_xor, lx, ly, lw, lh)?;
						}

						//Draw outline.
						if let Ok(geom) = xconnection.get_geometry(win)?.reply() {
							drawxoroutline(&xconnection, screen.root, gc_xor, new_x, new_y, geom.width, geom.height, )?;
							xordrawn = Some((new_x, new_y, geom.width, geom.height));
						}
					} else {
						xconnection.configure_window(
							win,
							&ConfigureWindowAux::new().x(new_x as i32).y(new_y as i32),
						)?;
					}
					}
				}
				
				
				
				
				
				
				//For releasing the window! Redraw the frame!
				Event::ButtonRelease(_) => {
					if FASTDRAG {
						//Draw XOR Outline to overwrite old one.
						if let Some((lx, ly, lw, lh)) = xordrawn { drawxoroutline(&xconnection, screen.root, gc_xor, lx, ly, lw, lh)?; xordrawn = None; }
					}
					//Move window.
					if let Some(target) = moving {
						if let (Some((finalx, finaly)), Some((targetx, targety))) = (drag, origin) {
							let pointer = xconnection.query_pointer(screen.root)?.reply()?;
							xconnection.configure_window(target, &ConfigureWindowAux::new().x((targetx + (pointer.root_x - finalx)) as i32).y((targety + (pointer.root_y - finaly)) as i32))?;
						}
					}

					moving = None;
					drag = None;
					origin = None;

					//Redraw window frames.
					redrawframes(&xconnection, screen, panel, titlebar, border, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext)?;

					xconnection.flush()?;
				}
				
				//We need a lot more comments here!
                Event::ButtonPress(press) => {
					
					
					if press.detail == 1 { //Left mouse button pressed.
					
						//For the outline drag only.
						if FASTDRAG {
							if let Event::ButtonPress(ButtonPressEvent { root_x, root_y, .. }) = event {
								let pointer = xconnection.query_pointer(screen.root)?.reply()?;
								dragpressoffsetx = pointer.root_x - root_x as i16;
								dragpressoffsety = pointer.root_y - root_y as i16;
							}
						}
					
					
					
					
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

							//Lets refactor this code. We'll have a simple trigger. refresh[window] = true, or something. All drawing will take place at the end of the loop.
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



							//Draw the taskbar window buttons.
							drawpanelwindows(&xconnection, panel, 61, width - notification - 67, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_highcheckers)?;
							//xconnection.flush()?;
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


















	

	






