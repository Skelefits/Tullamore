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
use std::collections::HashMap;

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
    order: u8,
}

pub struct WindowManager {
    windows: HashMap<Window, WindowState>,
    frames: HashMap<Window, Window>
}

impl WindowManager {
    pub fn new() -> Self {
        WindowManager {
            windows: HashMap::new(),
            frames: HashMap::new()
        }
    }
	
    pub fn focus<C: Connection>(&mut self, xconnection: &C, target: Window, panel: Window) -> Result<(), Box<dyn Error>> {
        xconnection.configure_window(target, &ConfigureWindowAux::default().sibling(panel).stack_mode(StackMode::BELOW))?;
        for state in self.windows.values_mut() {
            if state.frame == target {
                state.map = 2;  //Focus
            } else if state.map == 2 {
                state.map = 3;  //Old focused window is now just visible.
            }
        }

        Ok(())
    }

    // Original method
    pub fn getwindow(&self, window: &Window) -> Option<&WindowState> {
        self.windows.get(window)
    }

    pub fn findwindow<'a>(&'a self, title: &'a str) -> impl Iterator<Item = &'a WindowState> + 'a {
        self.windows.values().filter(move |w| w.title == title)
    }

	pub fn insertwindow(&mut self, state: WindowState) {
		let window = state.window;
		let frame = state.frame;
		self.frames.insert(frame, window);
		self.windows.insert(window, state);
	}

    pub fn getframe(&self, frame: &Window) -> Option<&WindowState> {
        self.frames.get(frame)
            .and_then(|window| self.windows.get(window))
    }

    pub fn removewindow(&mut self, window: &Window) {
        if let Some(state) = self.windows.remove(window) {
            self.frames.remove(&state.frame);
        }
    }
	
    pub fn fillblanks(&mut self) {
        let mut max = 0;
        let mut update = Vec::new();
        
        for (window, state) in self.windows.iter() {
            if state.order > max {
                max = state.order;
            } else if state.order == 0 {
                update.push(*window);
            }
        }
        
        let mut next = max + 1;
        for window in update {
            if let Some(state) = self.windows.get_mut(&window) {
                state.order = next;
                next += 1;
            }
        }
    }
	
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
	
    xconnection.create_pixmap(24, pixmap, window, 2, 2)?;

    xconnection.create_gc(temp_gc1, pixmap, &CreateGCAux::default().foreground(firstcolour))?;
    xconnection.create_gc(temp_gc2, pixmap, &CreateGCAux::default().foreground(secondcolour))?;

    xconnection.poly_fill_rectangle(pixmap, temp_gc1, &[
        Rectangle { x: 0, y: 0, width: 1, height: 1 },
        Rectangle { x: 1, y: 1, width: 1, height: 1 }
    ])?;
    xconnection.poly_fill_rectangle(pixmap, temp_gc2, &[
        Rectangle { x: 1, y: 0, width: 1, height: 1 },
        Rectangle { x: 0, y: 1, width: 1, height: 1 }
    ])?;

    xconnection.create_gc(gc, window, &CreateGCAux::default()
        .tile(pixmap)
        .fill_style(FillStyle::TILED)
    )?;

    xconnection.free_gc(temp_gc1)?;
    xconnection.free_gc(temp_gc2)?;
    xconnection.free_pixmap(pixmap)?;

    Ok(gc)
}

fn createwindow<C: Connection>(xconnection: &C, screen: &Screen, x: i16, y: i16, width: u16, height: u16, title: &[u8], screen_width: i16, screen_height: i16, border: u16, titlebar: u16, gc_highlight: Gcontext, gc_lowlight: Gcontext, gc_highbackground: Gcontext, gc_lowbackground: Gcontext, gc_titlebar: Gcontext, gc_titlebartext: Gcontext, windowmanager: &mut WindowManager) -> Result<Window, Box<dyn Error>> {
    let window = xconnection.generate_id()?;
    xconnection.create_window(COPY_DEPTH_FROM_PARENT, window, screen.root, x, y, width, height, 0, WindowClass::INPUT_OUTPUT, 0, &CreateWindowAux::new().background_pixel(screen.white_pixel).event_mask(EventMask::EXPOSURE | EventMask::BUTTON_PRESS))?;
    xconnection.change_property8(PropMode::REPLACE, window, AtomEnum::WM_NAME, AtomEnum::STRING, title)?;
    xconnection.change_window_attributes(window, &ChangeWindowAttributesAux::default().override_redirect(0))?;
	let frame = createborder(xconnection, screen, window, screen_width, screen_height, border, titlebar, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext)?;
    xconnection.map_window(window)?;
    let state = WindowState {window, frame, title: String::from_utf8_lossy(title).to_string(), x, y, z: 0, width: width as i16, height: height as i16, map: 2, order: 0};
    windowmanager.insertwindow(state);
    Ok(window)
}

fn desktop() -> Result<(), Box<dyn Error>> {
	//let width = 640 as i16;
	//let height = 480 as i16;
	
	let mut wm = WindowManager::new();
	
	
	
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

	//test windows
	
	let test1 = createwindow(&xconnection, &screen, 100, 100, 200, 100, b"test1", 1920, 1080, 2, 20, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext, &mut wm)?;
	
	let test2 = createwindow(&xconnection, &screen, 100, 100, 300, 200, b"test2", 1920, 1080, 2, 20, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext, &mut wm)?;
	
	let test3 = createwindow(&xconnection, &screen, 100, 100, 100, 100, b"test3", 1920, 1080, 2, 20, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext, &mut wm)?;

	wm.fillblanks();

	//Draw window boxes on the panel.
	drawpanelwindows(&xconnection, panel, 61, width - notification - 67, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_highcheckers, &wm)?;





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
							let newx = targetx + (pointer.root_x - finalx);
							let newy = targety + (pointer.root_y - finaly);
							
							//Update xy on X server.
							xconnection.configure_window(target, &ConfigureWindowAux::new().x(newx as i32).y(newy as i32))?;
							
							//Update wm!
							if let Some(state) = wm.windows.values_mut().find(|s| s.frame == target) {
								state.x = newx;
								state.y = newy;
							}
						}
					}

					moving = None;
					drag = None;
					origin = None;

					//Redraw window frames.
					redrawframes(&xconnection, &wm, panel, titlebar, border, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext)?;

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
						if let Some((frame, statex, statey)) = wm.getframe(&target).map(|state| (state.frame, state.x, state.y)).or_else(|| wm.windows.values().find(|state| state.window == target).map(|state| (state.frame, state.x, state.y))) {
							wm.focus(&xconnection, frame, panel)?;
							
							if target == frame && press.event_y < titlebar as i16 {
								moving = Some(frame);
								drag = Some((press.root_x, press.root_y));
								origin = Some((statex, statey));
							}

							let windows_to_update: Vec<(Window, Window, i16, i16)> = wm.windows.values().filter(|state| state.map == 2 || state.map == 3).map(|state| {
								let frame_width = state.width + border as i16;
								let frame_height = state.height + border as i16 + 2 + (titlebar as i16);
								(state.frame, state.window, frame_width, frame_height)
							}).collect();

							for (frame, client, width, height) in windows_to_update {
								if frame != panel {
									updateborder(&xconnection, frame, client, width, height, titlebar, border,gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground,gc_titlebar, gc_titlebartext)?;
								}
							}
							//Draw the taskbar window buttons.
							drawpanelwindows(&xconnection, panel, 61, width - notification - 67, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_highcheckers, &wm)?;
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