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

use crate::trundle::windowborder;
use crate::trundle::drawtitlebar;
use crate::trundle::grabwindowtitle;
use crate::trundle::drawtitletext;
use crate::trundle::squishtext;
use crate::drawdepressedbumpyframe;
use crate::drawpng;
use crate::drawbumpyframe;
use crate::WindowManager;


use crate::trundle::{
    COLOURS,
    HIGHBACKGROUND_COLOUR,
    LOWBACKGROUND_COLOUR,
    HIGHLIGHT_COLOUR,
    LOWLIGHT_COLOUR,
    WALLPAPER_COLOUR,
    TITLEBAR_COLOUR
};



pub fn redrawframes<C: Connection>(xconnection: &C, wm: &WindowManager, panel: Window, titlebar: u16, border: u16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32, gc_titlebar: u32, gc_titlebartext: u32) -> Result<(), Box<dyn Error>> {
    for state in wm.windows.values() {
        if state.frame != panel {
			//println!("[{}] Frame Details - Window: {:?}, Frame: {:?}, Title: '{}', Original Size: {}x{}, Border: {}, Titlebar: {}, Final Size: {}x{}", "Skelefits", state.window, state.frame, state.title, state.width, state.height, border, titlebar, state.width + (2 * border as i16), state.height + ((2 * border as i16) + titlebar as i16));
            updateborder(xconnection, state.frame, state.window, state.width + (2 * border as i16), state.height + ((2 * border as i16) + titlebar as i16), titlebar, border, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext )?;
        }
    }
    Ok(())
}

pub fn createwmborder<C: Connection>(xconnection: &C, screen: &Screen, wm: &WindowManager, target: Window, width: u16, height: u16, screen_width: i16, screen_height: i16, border: u16, titlebar: u16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32, gc_titlebar: u32, gc_titlebartext: u32) -> Result<Window, Box<dyn Error>> {
    if let Some(state) = wm.getwindow(&target) {
        let fwidth = (width + border + border) as i16;
        let fheight = (height + titlebar + border + border) as i16;

        let fx = (state.x - border as i16).max(0).min(screen_width - fwidth);
        let fy = (state.y - (titlebar as i16 - border as i16)).max(0).min(screen_height - fheight);

        let frame = xconnection.generate_id()?;
        xconnection.create_window(COPY_DEPTH_FROM_PARENT, frame, screen.root, fx, fy, fwidth as u16, fheight as u16, 0, WindowClass::INPUT_OUTPUT, 0, &CreateWindowAux::new().background_pixel(COLOURS[HIGHBACKGROUND_COLOUR]).event_mask(EventMask::EXPOSURE | EventMask::BUTTON_PRESS | EventMask::POINTER_MOTION | EventMask::BUTTON_RELEASE))?;
        xconnection.configure_window(target, &ConfigureWindowAux::new().border_width(0))?;
        xconnection.reparent_window(target, frame, border as i16, (border + titlebar) as i16)?;
        xconnection.map_window(frame)?;
        xconnection.map_window(target)?;
        updateborder(xconnection, frame, target, fwidth, fheight, titlebar, border, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext)?;
        xconnection.flush()?;
        Ok(frame)
    } else {
        Err("Window not found in window manager".into())
    }
}

pub fn createborder(xconnection: &impl x11rb::connection::Connection, screen: &x11rb::protocol::xproto::Screen, target: u32, width: i16, height: i16, border: u16, titlebar: u16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32, gc_titlebar: u32, gc_titlebartext: u32) -> Result<u32, Box<dyn std::error::Error>> {
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

        //dfupdateborder(xconnection, frame, target, fwidth as i16, fheight as i16, titlebar, border, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext, )?;

		xconnection.flush()?;
		Ok(frame)
    } else {
        Err("Failed to get geometry".into())
    }
}

pub fn updateborder<C: x11rb::connection::Connection>(xconnection: &C, frame: u32, target: u32, width: i16, height: i16, titlebar: u16, border: u16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32, gc_titlebar: u32, gc_titlebartext: u32) -> Result<(), Box<dyn std::error::Error>> {
    const TITLE_INSET: i16 = 8;
    const TEXT_Y_OFFSET: i16 = 1;
    let root = xconnection.setup().roots[0].root;
    let focused = if let Ok(root_tree) = xconnection.query_tree(root)?.reply() { root_tree.children.iter().rev().nth(1).map_or(false, |&w| w == frame) } else { false };

	//println!("{}x{}", width, height);
    windowborder(xconnection, frame, width, height, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground)?;
    let gc = if focused { gc_titlebar } else { gc_lowbackground };
    drawtitlebar(xconnection, frame, width - TITLE_INSET, titlebar as i16, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc)?;
    let gc = if focused { gc_titlebartext } else { gc_highbackground };
	drawtitletext(xconnection, frame, gc, target, TITLE_INSET, titlebar as i16 - TEXT_Y_OFFSET)?;
    Ok(())
}

pub fn drawpanelwindows<C: Connection>(xconnection: &C, window: u32, startx: i16, workingwidth: i16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32, gc_highcheckers: u32, wm: &WindowManager) -> Result<(), Box<dyn Error>> {
    const WINDOW_TARGET_WIDTH: i16 = 160;
    const WINDOW_MIN_WIDTH: i16 = 30;
    const WINDOW_SPACING: i16 = 3;
    const ICON_WIDTH: u16 = 16;
    const ICON_HEIGHT: u16 = 16;
    let mut windows: Vec<(Window, String, Window)> = wm.windows.values().filter(|state| state.map == 2 || state.map == 3).map(|state| (state.frame, state.title.clone(), state.window)).collect();
    if windows.is_empty() { return Ok(()); }
    windows.sort_by_key(|(frame, _, _)| wm.windows.values().find(|state| state.frame == *frame).map(|state| state.order).unwrap_or(0));
    let windowscount = windows.len() as i16;
    let requiredwidth = (WINDOW_TARGET_WIDTH * windowscount) + (WINDOW_SPACING * (windowscount - 1));
    let finalwidth = if requiredwidth > workingwidth { ((workingwidth - (WINDOW_SPACING * (windowscount - 1))) / windowscount).max(WINDOW_MIN_WIDTH) } else { WINDOW_TARGET_WIDTH };
    let focused = wm.windows.values().find(|state| state.map == 2).map(|state| state.frame);
    let mut offset = startx;
    for (i, (frameid, title, clientid)) in windows.iter().enumerate() {
        let max_chars = ((finalwidth - 20) / 6).max(1);
        let display_title = if title.len() > max_chars as usize {
            if max_chars > 3 { format!("{}...", &title[0..(max_chars - 3) as usize]) } else { title[0..max_chars as usize].to_string() }
        } else {
            title.clone()
        };
        if Some(*frameid) == focused {
            drawdepressedbumpyframe(xconnection, window, offset, 4, finalwidth, 21, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_highcheckers)?;
            drawpng(xconnection, window, "computer.png", offset + 4, 8, ICON_WIDTH, ICON_HEIGHT, COLOURS[HIGHBACKGROUND_COLOUR])?;
            if !display_title.is_empty() { xconnection.image_text8(window, gc_lowlight, offset + 22, 20, display_title.as_bytes())?; }
        } else {
            drawbumpyframe(xconnection, window, offset, 4, finalwidth, 21, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground)?;
            drawpng(xconnection, window, "computer.png", offset + 4, 7, ICON_WIDTH, ICON_HEIGHT, COLOURS[HIGHBACKGROUND_COLOUR])?;
            if !display_title.is_empty() { xconnection.image_text8(window, gc_lowlight, offset + 22, 19, display_title.as_bytes())?; }
        }
        offset += finalwidth + WINDOW_SPACING;
        if offset + finalwidth > startx + workingwidth { eprintln!("Uh-oh! We are out of space at Window {}!", i+1); break; }
    }
    xconnection.flush()?;
    Ok(())
}

pub fn drawwindowbuttons<C: Connection>(xconnection: &C, panel: Window, u8_panelitem: u8, str_title: &str, x: i16, width: i16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32, gc_highcheckers: u32) -> Result<(), Box<dyn Error>> {
	println!("u8_panelitem {}", u8_panelitem);	
    if u8_panelitem == 43 {
        drawdepressedbumpyframe(xconnection, panel, x, 4, width, 21, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_highcheckers)?;
		if width >= 20 {
			drawpng(xconnection, panel, "computer.png", x + 4, 8, 16, 16, COLOURS[HIGHBACKGROUND_COLOUR])?;
			xconnection.image_text8(panel, gc_lowlight, x + 24, 20, squishtext(str_title, width - 28, 6).as_bytes())?;
		}
    } else {
		if u8_panelitem == 41 {
			drawdepressedbumpyframe(xconnection, panel, x, 4, width, 21, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, 0)?;
		} else {
			drawbumpyframe(xconnection, panel, x, 4, width, 21, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground)?;
		}
		if width >= 20 {
			drawpng(xconnection, panel, "computer.png", x + 4, 7, 16, 16, COLOURS[HIGHBACKGROUND_COLOUR])?;
			xconnection.image_text8(panel, gc_lowlight, x + 24, 19, squishtext(str_title, width - 28, 6).as_bytes())?;
		}
    }
    Ok(())
}