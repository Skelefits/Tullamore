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
use window::createwmborder;
use window::drawwindowbuttons;
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
use trundle::squishtext;

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

fn grabexternalwindows<C: Connection>(xconnection: &C, wm: &mut WindowManager, root_window: Window,) -> Result<(), Box<dyn Error>> {
    let tree = xconnection.query_tree(root_window)?.reply()?;
    for window in tree.children {
        if wm.getwindow(&window).is_some() || wm.frames.contains_key(&window) {
        } else if let Ok(attributes) = xconnection.get_window_attributes(window)?.reply() {
            if attributes.map_state == MapState::VIEWABLE && !attributes.override_redirect {
                if let Ok(geometry) = xconnection.get_geometry(window)?.reply() {
                    let title = xconnection.get_property(false, window, AtomEnum::WM_NAME, AtomEnum::STRING, 0, u32::MAX,)?.reply().ok().and_then(|prop| String::from_utf8(prop.value).ok()).unwrap_or_else(|| String::from("Unknown"));
					//I think we may be able to get away with eventually removing the below line.
					wm.installexternalwindow(window, window, title, geometry.x, geometry.y, geometry.width as i16, geometry.height as i16, 0);
                }
            }
        }
    }
    Ok(())
}

impl WindowManager {
    pub fn new() -> Self {
        WindowManager {
            windows: HashMap::new(),
            frames: HashMap::new()
        }
    }
	
	pub fn installexternalwindow(&mut self, window: Window, frame: Window, title: String, x: i16, y: i16, width: i16, height: i16, order: u8) {
		let state = WindowState {window, frame, title, x, y, z: 0, width: width as i16, height: height as i16, map: 2, order,};
		self.insertwindow(state);
	}
	
    pub fn focus<C: Connection>(&mut self, xconnection: &C, target: Window, panel: Window) -> Result<(), Box<dyn Error>> {
        xconnection.configure_window(target, &ConfigureWindowAux::default().sibling(panel).stack_mode(StackMode::BELOW))?;
        for state in self.windows.values_mut() {
            if state.frame == target {
                state.map = 2; //Focus
            } else if state.map == 2 {
                state.map = 3; //Old focused window is now just visible.
            }
        }

        Ok(())
    }

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


fn main() -> Result<(), Box<dyn Error>> {
    //let handle = thread::spawn(|| { //async this maybe, or remove
        if let Err(e) = desktop() {
            eprintln!("Error in panel: {}", e);
        }
    //});
    //handle.join().unwrap();

    Ok(())
}

fn drawchunkyxoroutline<C: Connection>(xconnection: &C, window: u32, gc: u32, x: i16, y: i16, width: u16, height: u16,) -> Result<(), Box<dyn Error>> {
    const THICKNESS: u16 = 4;
    let screen = xconnection.setup().roots.first().unwrap();
    let screen_height = screen.height_in_pixels as i16;
    let panely = screen_height - 28;

    let mut rectangles = Vec::new();
    if y < panely {
        let visible_top_height = (panely - y).min(THICKNESS as i16).max(0) as u16;
        if visible_top_height > 0 {
            rectangles.push(Rectangle { x, y, width, height: visible_top_height });
        }
    }
    let bottom_y = y + height as i16;
    if bottom_y > panely {
        let visible_bottom_height = (THICKNESS as i16 - (bottom_y - panely)).max(0) as u16;
        if visible_bottom_height > 0 {
            rectangles.push(Rectangle { x, y: panely - visible_bottom_height as i16, width, height: visible_bottom_height });
        }
    } else {
        rectangles.push(Rectangle { x, y: bottom_y - THICKNESS as i16, width, height: THICKNESS });
    }

    let visible_left_height = ((height as i16 - 2 * THICKNESS as i16).min(panely - y - THICKNESS as i16)).max(0);
    if visible_left_height > 0 {
        rectangles.push(Rectangle { x, y: y + THICKNESS as i16, width: THICKNESS, height: visible_left_height as u16 });
        rectangles.push(Rectangle { x: x + width as i16 - THICKNESS as i16, y: y + THICKNESS as i16, width: THICKNESS, height: visible_left_height as u16 });
    }
    if !rectangles.is_empty() {
        xconnection.poly_fill_rectangle(window, gc, &rectangles)?;
    }
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

fn makexorpattern<C: Connection>(xconnection: &C, window: u32) -> Result<u32, Box<dyn Error>> {
    let pixmap = xconnection.generate_id()?;
    let gc = xconnection.generate_id()?;
    let gc_temp = xconnection.generate_id()?;
    
    xconnection.create_pixmap(24, pixmap, window, 2, 2)?;

    xconnection.create_gc(gc_temp, pixmap, &CreateGCAux::default().foreground(0x000000))?;
    xconnection.poly_fill_rectangle(pixmap, gc_temp, &[Rectangle { x: 0, y: 0, width: 2, height: 2 }])?;

    xconnection.change_gc(gc_temp, &ChangeGCAux::default().foreground(0xFFFFFF))?;
    xconnection.poly_fill_rectangle(pixmap, gc_temp, &[
        Rectangle { x: 0, y: 0, width: 1, height: 1 },
        Rectangle { x: 1, y: 1, width: 1, height: 1 },
    ])?;

    xconnection.create_gc(gc, window, &CreateGCAux::default().tile(pixmap).fill_style(FillStyle::TILED).function(Some(GX::XOR)).foreground(0xFFFFFF).subwindow_mode(SubwindowMode::INCLUDE_INFERIORS),)?;

    xconnection.free_gc(gc_temp)?;
    xconnection.free_pixmap(pixmap)?;

    Ok(gc)
}

fn createwindow<C: Connection>(xconnection: &C, screen: &Screen, x: i16, y: i16, width: u16, height: u16, title: &[u8], reswidth: i16, resheight: i16, border: u16, titlebar: u16, gc_highlight: Gcontext, gc_lowlight: Gcontext, gc_highbackground: Gcontext, gc_lowbackground: Gcontext, gc_titlebar: Gcontext, gc_titlebartext: Gcontext, windowmanager: &mut WindowManager) -> Result<Window, Box<dyn Error>> {
    let window = xconnection.generate_id()?;
    xconnection.create_window(COPY_DEPTH_FROM_PARENT, window, screen.root, x, y, width, height, 0, WindowClass::INPUT_OUTPUT, 0, &CreateWindowAux::new().background_pixel(screen.white_pixel).event_mask(EventMask::EXPOSURE | EventMask::BUTTON_PRESS))?;
    xconnection.change_property8(PropMode::REPLACE, window, AtomEnum::WM_NAME, AtomEnum::STRING, title)?;
    xconnection.change_window_attributes(window, &ChangeWindowAttributesAux::default().override_redirect(0))?;
	let frame = createborder(xconnection, screen, window, reswidth, resheight, border, titlebar, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext)?;
    xconnection.map_window(window)?;
    let state = WindowState {window, frame, title: String::from_utf8_lossy(title).to_string(), x, y, z: 0, width: width as i16, height: height as i16, map: 2, order: 0};
    windowmanager.insertwindow(state);
	

	
    Ok(window)
}

fn definepanelitems(panelitems: &mut [[u8; 1]; 128], panelcoordinates: &mut [[i16; 2]; 128], panelwindows: &mut [[u32; 1]; 128], width: i16, icons: u8, panelindex: &mut [u8; 6]) {
    //Type and Actions
	// 1 = Start Button Ready
	// 2 = Start Button Pressed
	//10 = Task List
	//20 = Quick Laucher
	//30 = Icon Link
	//31 = Icon Link Pressed
	//32 = Icon Link Hover
	
	//40 = Taskbar Button Ready
	//41 = Taskbar Button Pressed
	//50 = Taskbar Button Arrows
	//60 = Notification Area
	
	//[Type and Action, Window (if under 255, links to another array which will define icon and link)]
	panelitems[0] = [1]; //Start button!
	//[X, Width]
	panelcoordinates[0] = [2, 54];
	panelwindows[0] = [0];
	panelindex[0] = 0;
	
	//The icons start at 60, and are 23 wide each.
	
	
	//panelitems[4] = [40]; 
	//panelcoordinates[4] = [120, 100];
	//panelwindows[4] = [999];
	
	//panelitems[5] = [40];
	//panelcoordinates[5] = [230, 100];
	//panelwindows[5] = [999];
	
	
	//panelitems[6] = [41];
	//panelcoordinates[6] = [340, 100];
	//panelwindows[6] = [999];
	
	let notification = ((icons as i16 *20) + 60) as i16; //took ages to work out icons was causing a buffer overflow

	panelindex[1] = 0;
	panelindex[2] = 0;
	panelindex[3] = 0;
	panelindex[4] = 0;
	panelindex[5] = 1;
	
	//LINKSTART 1;
	//LINKEND 2;
	//WINDOWSTART 3;
	//WINDOWEND 4;
	//NOTIFICATIONSTART 5;
	


	println!("Width ({}) Coordinates ({}) Notification ({}) Icons ({})", width, width - notification - 3, notification, icons);

	panelitems[panelindex[5] as usize] = [60]; //Notification Area
	panelcoordinates[panelindex[5] as usize] = [width - notification - 3, notification];
	panelwindows[panelindex[5] as usize] = [0];
}

fn definepanelicons(link: &mut [[String; 4]; 32], icons:  &mut u8) {
	//0-15 is for quick links in the task bar.
	//16-31 is for icons in the tray.
	
	
	link[0][0] = "".to_string(); //Program to open, nul means Tullamore is hardcoded with an action.
	link[0][1] = "Start".to_string(); //Text
	link[0][2] = "Click here to begin.".to_string(); //Tool Tip
	link[0][3] = "computer.png".to_string(); //Icon
	
	link[1] = [String::from(""), "Web Browser".to_string(), "The Internet is for the weak!".to_string(), "audio-volume-muted.png".to_string()];
	link[2] = [String::from(""), "Web Browser".to_string(), "The Internet is for the weak!".to_string(), "network-offline.png".to_string()];
	link[3] = [String::from(""), "Web Browser".to_string(), "The Internet is for the weak!".to_string(), "weather-snow.png".to_string()];
	
	
	link[31] = [String::from(""), "Sound".to_string(), "Sound Muted".to_string(), "audio-volume-muted.png".to_string()];
	link[30] = [String::from(""), "Network".to_string(), "Network Offline".to_string(), "network-offline.png".to_string()];
	link[29] = [String::from(""), "Weather".to_string(), "Snowing".to_string(), "weather-snow.png".to_string()];
	

	*icons = 3;
}

fn updatepanelindex(index: usize, panelindex: &mut [u8; 6]) {
	//This deals with updating the indexes. It is a lot more complication due to initialization.
    if index == 2 {
        panelindex[1] = 1;
    }
    match index {
        2 => {
            panelindex[2] += 1;
            panelindex[3] = panelindex[2];
            panelindex[4] = panelindex[2];
            panelindex[5] = panelindex[2] + 1;
        },
        4 => {
            if panelindex[3] == panelindex[2] {
                panelindex[3] = panelindex[2] + 1;
                panelindex[4] = panelindex[3];
                panelindex[5] = panelindex[4] + 1;
            } else {
                panelindex[4] += 1;
                panelindex[5] = panelindex[4] + 1;
            }
        },
        _ => {}
    }

    println!("panelindex contents: [0]={}, [1]={}, [2]={}, [3]={}, [4]={}, [5]={}", 
        panelindex[0], panelindex[1], panelindex[2], panelindex[3], panelindex[4], panelindex[5]);
}

fn shiftpanelicon(index: usize, panelindex: &mut [u8; 6], panelitems: &mut [[u8; 1]; 128], panelcoordinates: &mut [[i16; 2]; 128], panelwindows: &mut [[u32; 1]; 128], panelicons: &mut [[String; 4]; 32]) {

	//This function does not adjust panelindex for everything. They should be updated before calling this function.

    //Move everything from index to the right.
	
	

	
    for i in (index..panelindex[5] as usize).rev() {
        panelitems[i + 1] = panelitems[i];
		panelcoordinates[i + 1][0] = panelcoordinates[i][0];

			
        if panelitems[i][0] < 60 as u8 {
            panelcoordinates[i + 1][0] = panelcoordinates[i][0] + 23;
        } else {
            panelcoordinates[i + 1][0] = panelcoordinates[i][0];
        }
		panelcoordinates[i + 1][1] = panelcoordinates[i][1];
        panelwindows[i + 1] = panelwindows[i];
    }
	//Clear index.
    panelitems[index] = [0];
	if index > 0 {
		//New coords are previous index's start location plus width.
		if index == 1 {
			//First icon next to the start button.
			panelcoordinates[index][0] = 60;
		} else {
			panelcoordinates[index][0] = panelcoordinates[index - 1][0] + panelcoordinates[index - 1][1];
		}
		
	}
    panelwindows[index] = [0];
	

}

fn insertpanelicon(index: usize, panelindex: &mut [u8; 6], icon: u32, panelitems: &mut [[u8; 1]; 128], panelcoordinates: &mut [[i16; 2]; 128], panelwindows: &mut [[u32; 1]; 128], panelicons: &mut [[String; 4]; 32]) {
	//let px = panelcoordinates[index - 1][0] + panelcoordinates[index - 1][1];
	//
	
	updatepanelindex(2, panelindex); //Increment indexes from the end of the link area.
	
	shiftpanelicon(index, panelindex, panelitems, panelcoordinates, panelwindows, panelicons);
    panelitems[index] = [30];
    panelcoordinates[index] = [panelcoordinates[index][0], 23];
    panelwindows[index] = [icon];
	
}


fn insertpanelwindow(panelindex: &mut [u8; 6], window: u32, panelitems: &mut [[u8; 1]; 128], panelcoordinates: &mut [[i16; 2]; 128], panelwindows: &mut [[u32; 1]; 128], panelicons: &mut [[String; 4]; 32]) {
	//let px = panelcoordinates[index - 1][0] + panelcoordinates[index - 1][1];
	//
	
	updatepanelindex(4, panelindex); //Increment indexes from the end of the window area.
	let index = panelindex[4] as usize + 1;
    shiftpanelicon(index, panelindex, panelitems, panelcoordinates, panelwindows, panelicons);
    panelitems[index] = [40];
    panelwindows[index] = [window];

    let tray = panelcoordinates[panelindex[5] as usize][0];
    
    if panelcoordinates[index][0] + 160 < tray {
        panelcoordinates[index] = [panelcoordinates[index][0] + 3, 160];
    } else {

        //Window buttons go into the notification area. Let's calculate a new width for them.
        let startx = panelcoordinates[panelindex[3] as usize][0];
        let endx = tray - 9;
        let width = (endx - startx) / (3 + panelindex[5] as i16 - panelindex[3] as i16);
        
        let mut x = startx;
        for i in panelindex[3]..panelindex[5] {
            panelcoordinates[i as usize] = [x, width];
            x += width + 3;
        }
    }
	
}





fn addpanelicon(tray: u8, label: String, tooltip: String, icon: String, link: &mut [[String; 4]; 32], panelindex: &mut [u8; 6], panelitems: &mut [[u8; 1]; 128], panelcoordinates: &mut [[i16; 2]; 128], panelwindows: &mut [[u32; 1]; 128]) {
	//Will do nothing if out of space. Probably want to return an error or something.
    //Add to Quick Links!
    for i in 0..15 {
        if link[i][3].is_empty() {
            link[i] = [String::from(""), label, tooltip, icon];
            insertpanelicon(tray as usize, panelindex, i as u32, panelitems, panelcoordinates, panelwindows, link);
            break;
        }
    }
}

fn addnotificationicon(label: String, tooltip: String, icon: String, link: &mut [[String; 4]; 32], icons: &mut u8) {
	//Will do nothing if out of space. Probably want to return an error or something.
    for i in (16..32).rev() {
        if link[i][3].is_empty() {
            link[i] = [String::from(""), label, tooltip, icon];
            *icons += 1;
            break;
        }
    }
}



fn desktop() -> Result<(), Box<dyn Error>> {
	//let width = 640 as i16;
	//let height = 480 as i16;
	
	let mut wm = WindowManager::new();
	
    let (xconnection, screenid) = x11rb::connect(Some(":0"))?;
    let screen = &xconnection.setup().roots[screenid];
	
	let width = screen.width_in_pixels as i16;
	let height = screen.height_in_pixels as i16;
	
	
	//calculate the size of the notification box.
	

	
	//What links and notification tray icons do we have?
	let mut panelicons: [[String; 4]; 32] = Default::default();
	let mut icons = 0 as u8;
	definepanelicons(&mut panelicons, &mut icons);
	



	
	
	let mut panelitems: [[u8; 1]; 128] = [[0; 1]; 128];
	let mut panelcoordinates: [[i16; 2]; 128] = [[0; 2]; 128];
	let mut panelwindows: [[u32; 1]; 128] = [[0; 1]; 128];

	addnotificationicon("yo".to_string(), "yo".to_string(), "computer.png".to_string(), &mut panelicons, &mut icons,);
	addnotificationicon("yo".to_string(), "yo".to_string(), "computer.png".to_string(), &mut panelicons, &mut icons,);

	//let mut trayindex = 7 as u8;
	let mut panelindex: [u8; 6] = [0; 6];
	
	
	
	definepanelitems(&mut panelitems, &mut panelcoordinates, &mut panelwindows, width, icons, &mut panelindex);
	
	
	addpanelicon(panelindex[2], "yo".to_string(), "yo".to_string(), "computer.png".to_string(), &mut panelicons, &mut panelindex, &mut panelitems, &mut panelcoordinates, &mut panelwindows);
	addpanelicon(panelindex[2], "yo".to_string(), "yo".to_string(), "computer.png".to_string(), &mut panelicons, &mut panelindex, &mut panelitems, &mut panelcoordinates, &mut panelwindows);
	addpanelicon(panelindex[2], "yo".to_string(), "yo".to_string(), "computer.png".to_string(), &mut panelicons, &mut panelindex, &mut panelitems, &mut panelcoordinates, &mut panelwindows);
	
	
	
	
    //let mut panellinks: [[String; 4]; 16] = [[String; 4]; 16];
    //definepanellinks(&mut panellinks);
	

	
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
	let gc_xorcheckers = xconnection.generate_id()?;

    //Show window.
    xconnection.map_window(panel)?;
	
	
	
	xconnection.create_gc(gc_lowlight, window, &CreateGCAux::default().foreground(COLOURS[LOWLIGHT_COLOUR]).background(COLOURS[HIGHBACKGROUND_COLOUR]))?;
	xconnection.create_gc(gc_highbackground, window, &CreateGCAux::default().foreground(COLOURS[HIGHBACKGROUND_COLOUR]).background(COLOURS[LOWBACKGROUND_COLOUR]))?;
	xconnection.create_gc(gc_lowbackground, window, &CreateGCAux::default().foreground(COLOURS[LOWBACKGROUND_COLOUR]))?;
	xconnection.create_gc(gc_highlight, window, &CreateGCAux::default().foreground(COLOURS[HIGHLIGHT_COLOUR]))?;
	xconnection.create_gc(gc_titlebar, window, &CreateGCAux::default().foreground(COLOURS[TITLEBAR_COLOUR]).background(COLOURS[HIGHBACKGROUND_COLOUR]))?;
	xconnection.create_gc(gc_titlebartext, window, &CreateGCAux::default().foreground(COLOURS[HIGHLIGHT_COLOUR]).background(COLOURS[TITLEBAR_COLOUR]))?;
	xconnection.create_gc(gc_xor, window, &CreateGCAux::default().function(Some(GX::XOR)).foreground(0xFFFFFF).subwindow_mode(SubwindowMode::INCLUDE_INFERIORS),)?;

	let gc_highcheckers = makepattern(&xconnection, window, COLOURS[HIGHLIGHT_COLOUR].unwrap(), COLOURS[HIGHBACKGROUND_COLOUR].unwrap())?;
	let gc_lowcheckers = makepattern(&xconnection, window, COLOURS[LOWLIGHT_COLOUR].unwrap(), COLOURS[LOWBACKGROUND_COLOUR].unwrap())?;

	let gc_xorcheckers = makexorpattern(&xconnection, window)?;
	
	
    let border = 4 as u16;
    let titlebar = 18 as u16;

	//test windows
	
	let test1 = createwindow(&xconnection, &screen, 100, 100, 200, 100, b"test1", width, height, 4, 18, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext, &mut wm)?;
	
	let test2 = createwindow(&xconnection, &screen, 100, 100, 300, 200, b"test2", width, height, 4, 18, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext, &mut wm)?;
	
	let test3 = createwindow(&xconnection, &screen, 100, 100, 100, 100, b"test3", width, height, 4, 18, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext, &mut wm)?;

	println!("test1 {} test2 {} test3 {}", test1, test2, test3);


	insertpanelwindow(&mut panelindex, test1, &mut panelitems, &mut panelcoordinates, &mut panelwindows, &mut panelicons);
	insertpanelwindow(&mut panelindex, test2, &mut panelitems, &mut panelcoordinates, &mut panelwindows, &mut panelicons);
	insertpanelwindow(&mut panelindex, test3, &mut panelitems, &mut panelcoordinates, &mut panelwindows, &mut panelicons);


	wm.fillblanks();

	redrawframes(&xconnection, &wm, panel, titlebar, border, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext)?;

	
	





	//Draw window boxes on the panel.
	//drawpanelwindows(&xconnection, panel, 61, width - notification - 67, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_highcheckers, &wm)?;







	let clockheight = 20;
	let (mut epoch, mut pminute, mut phour) = drawclock(&xconnection, panel, gc_lowlight, width, clockheight)?;

	
	



	//Put panel on top.
	xconnection.configure_window(panel, &ConfigureWindowAux::default().stack_mode(StackMode::ABOVE))?;

    xconnection.flush()?;








	let mut moving: Option<u32> = None;
	let mut drag: Option<(i16, i16)> = None;
	let mut origin: Option<(i16, i16)> = None;

	let mut dragpressoffsetx = 0;
	let mut dragpressoffsety = 0;

	let mut xordrawn: Option<(i16, i16, u16, u16)> = None;























	let mut draw = 1;




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
					
					let external = if let Ok(attributes) = xconnection.get_window_attributes(target.window)?.reply() {
						!attributes.override_redirect && !wm.getwindow(&target.window).is_some() && !wm.frames.contains_key(&target.window)
					} else {
						false
					};

					if external {
						if let Ok(geom) = xconnection.get_geometry(target.window)?.reply() {
							println!("Window Geometry Details:");
							println!("  x: {}, y: {}", geom.x, geom.y);
							println!("  width: {}, height: {}", geom.width as i16, geom.height as i16);
							println!("  border_width: {}", geom.border_width);
							println!("  depth: {}", geom.depth);
							println!("  root: {:?}", geom.root);
							
							let title = xconnection.get_property(false, target.window, AtomEnum::WM_NAME, AtomEnum::STRING, 0, u32::MAX)?.reply().ok().and_then(|prop| String::from_utf8(prop.value).ok()).unwrap_or_else(|| String::from("Unknown"));
							println!("  title: {}", title);
							
							wm.installexternalwindow(target.window, target.window, title, geom.x, geom.y, geom.width as i16, geom.height as i16, 0);
							insertpanelwindow(&mut panelindex, target.window, &mut panelitems, &mut panelcoordinates, &mut panelwindows, &mut panelicons);

							if let Ok(frame) = createwmborder(&xconnection, &screen, &wm, target.window, geom.width, geom.height, width, height, border, titlebar, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext) {
								if let Some(state) = wm.windows.get_mut(&target.window) {
									state.frame = frame;
								}
								wm.frames.insert(frame, target.window);
							}
						}
					}
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
							drawchunkyxoroutline(&xconnection, screen.root, gc_xorcheckers, lx, ly, lw, lh)?;
						}

						//Draw outline.
						if let Ok(geom) = xconnection.get_geometry(win)?.reply() {
							drawchunkyxoroutline(&xconnection, screen.root, gc_xorcheckers, new_x, new_y, geom.width, geom.height, )?;
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
						if let Some((lx, ly, lw, lh)) = xordrawn { drawchunkyxoroutline(&xconnection, screen.root, gc_xorcheckers, lx, ly, lw, lh)?; xordrawn = None; }
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

							let redraw: Vec<(Window, Window, i16, i16)> = wm.windows.values().filter(|state| state.map == 2 || state.map == 3).map(|state| {
								let fwidth = state.width + (2 * border as i16);
								let fheight = state.height + (2 * border as i16) + (titlebar as i16);
								(state.frame, state.window, fwidth, fheight)
							}).collect();

							for (frame, client, width, height) in redraw {
								if frame != panel {
									updateborder(&xconnection, frame, client, width, height, titlebar, border,gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground,gc_titlebar, gc_titlebartext)?;
								}
							}
							//Draw the taskbar window buttons.
							//drawpanelwindows(&xconnection, panel, 61, width - panelcoordinates[trayindex as usize][1] - 67, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_highcheckers, &wm)?;
							draw = 40;
						}
					
					
					
					
					
					
					
					
						if press.event_y > height - 20 && press.event_y < height - 5 {
							if press.event_x > width - panelcoordinates[panelindex[5] as usize][1] {
								if press.event_x < width - 60 {
									let clicked_icon = icons as i16 - 1 - (press.event_x - (width - panelcoordinates[panelindex[5] as usize][1])) / 20;
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
                Event::Error(_) => println!("bug bug"), _ => (),
            }

			
			
	if draw > 0 {


		if draw == 40 {
			//Redraw the tray windows only!
			for i in (1..(panelindex[5] as usize)).rev() {
				match panelitems[i][0] {
					40 => {
						//startx: i16, starty: i16, framewidth: i16, frameheight: i1
						drawwindowbuttons(&xconnection, panel, panelwindows[i][0], panelcoordinates[i][0], panelcoordinates[i][1], &wm, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_highcheckers)?;
					}
					_ => {
						break;
						
					}
				}
			}
		} else {

		//Let's draw the panel.
		xconnection.poly_fill_rectangle(panel, gc_highbackground, &[Rectangle { x: 0, y: 0, width: width as u16, height: panelheight }])?; //Draw panel background.
		xconnection.poly_line(CoordMode::PREVIOUS, panel, gc_highlight, &[Point { x: 0, y: 1 }, Point { x: width as i16, y: 0 }])?; //Draw panel highlight.


			//We will loop through all items in panel[?] and display them.
			for i in 0..(panelindex[5] as usize + 1) {
				match panelitems[i][0] {
					0 => {
						//Invalid panel item, assume the rest are too!
						break;
					}
					1 => {
						//Start Button
						drawstartbutton(&xconnection, panel, panelcoordinates[i][0], 4, panelcoordinates[i][1], 21, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground)?;
					}
					30 => {
						//Three pixels to the left of the icon, four to the right.
						drawpng(&xconnection, panel, &panelicons[panelwindows[i][0] as usize][3], panelcoordinates[i][0] + 3, 7, 16, 16, COLOURS[HIGHBACKGROUND_COLOUR])?;
						
					}
					40 => {
						//startx: i16, starty: i16, framewidth: i16, frameheight: i1
						drawwindowbuttons(&xconnection, panel, panelwindows[i][0], panelcoordinates[i][0], panelcoordinates[i][1], &wm, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_highcheckers)?;


						
					}
					60 => {
						//Notification Area
						
						//Notification - Depression
						drawdepressedframe(&xconnection, panel, panelcoordinates[i][0] + panelcoordinates[i][1], 4, panelcoordinates[i][1], 21, gc_highlight, gc_lowbackground)?;
						
						
						//Load notification graphic. 
						
						println!("Here Here {} {}", panelcoordinates[i][0], panelcoordinates[i][1]);
						
						
										
						let mut start = 16;
						//This is dumb. Surely there was a better way to print icons right to left.
						//Anyway, this bit of code will hopefully save something.
						//Reorder this when the default number of expected icons increases.
						if panelicons[27][3].is_empty() {
							start = 28;
						} else if panelicons[23][3].is_empty() {
							start = 24;
						} else if panelicons[19][3].is_empty() {
							start = 20;
						}
						//Check panelicons from Start to 31 and print them right to left in the notificaiton tray.
						let mut counter = 0;
						for b in start..=31 {
							if !panelicons[b][3].is_empty() {
								println!("Tray Icons {} {}", counter, b);
								drawpng(&xconnection, panel, &panelicons[b][3], panelcoordinates[i][0] + 3 + (counter * 20), 7, 16, 16, COLOURS[HIGHBACKGROUND_COLOUR])?;
								counter += 1;
							}
						}
						(_, pminute, phour) = drawclock(&xconnection, panel, gc_lowlight, width, clockheight)?;

					}
					_ => {
						println!("{} {} {} {}", panelitems[i][0], panelcoordinates[i][0], panelcoordinates[i][1], panelwindows[i][0]);
					}
				}
			}
		}
		draw = 0;
		//thread::sleep(Duration::from_millis(10));
	}
			
			
		xconnection.flush()?;
    }
    println!("wat?");
}