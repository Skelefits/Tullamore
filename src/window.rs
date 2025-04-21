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
use crate::drawdepressedbumpyframe;
use crate::drawpng;
use crate::drawbumpyframe;

use crate::trundle::{
    COLOURS,
    HIGHBACKGROUND_COLOUR,
    LOWBACKGROUND_COLOUR,
    HIGHLIGHT_COLOUR,
    LOWLIGHT_COLOUR,
    WALLPAPER_COLOUR,
    TITLEBAR_COLOUR
};



pub fn redrawframes<C: Connection>(xconnection: &C, screen: &Screen, panel: Window, titlebar: u16, border: u16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32, gc_titlebar: u32, gc_titlebartext: u32) -> Result<(), Box<dyn Error>> {
    if let Ok(root_tree) = xconnection.query_tree(screen.root)?.reply() {
        for &target in &root_tree.children {
            if target != panel {
                if let Ok(tree) = xconnection.query_tree(target)?.reply() {
                    //Window has children? Its a frame!
                    if !tree.children.is_empty() {
                        if let Ok(geom) = xconnection.get_geometry(target)?.reply() {
                            let width = geom.width as i16;
                            let height = geom.height as i16;
                            //Redraw the frame.
                            updateborder(xconnection, target, tree.children.last().copied().unwrap_or(target), width, height, titlebar, border, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext)?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn createborder(xconnection: &impl x11rb::connection::Connection, screen: &x11rb::protocol::xproto::Screen, target: u32, width: i16, height: i16, border: u16, titlebar: u16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32, gc_titlebar: u32, gc_titlebartext: u32) -> Result<(), Box<dyn std::error::Error>> {
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

pub fn updateborder<C: x11rb::connection::Connection>(xconnection: &C, frame: u32, target: u32, width: i16, height: i16, titlebar: u16, border: u16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32, gc_titlebar: u32, gc_titlebartext: u32) -> Result<(), Box<dyn std::error::Error>> {

    let root = xconnection.setup().roots[0].root;
    let mut is_focused = false;

    if let Ok(root_tree) = xconnection.query_tree(root)?.reply() {
        for &w in root_tree.children.iter().rev().skip(1) {
            if w == frame {
                is_focused = true;
                break;
            }
            break;
        }
    }


	windowborder(xconnection, frame, width, height, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground)?;
    //if height > (titlebar + 2 * border) as i16 {
		
	if is_focused {
		drawtitlebar(xconnection, frame, width - 8, titlebar as i16, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar)?;
	} else {
		drawtitlebar(xconnection, frame, width - 8, titlebar as i16, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_lowbackground)?;
	}
		

		
        
        drawtitletext(xconnection, frame, gc_titlebartext, target, 8, (titlebar as i16) - 1)?;
    //}
    Ok(())
}

pub fn drawpanelwindows<C: Connection>(xconnection: &C, window: u32, startx: i16, workingwidth: i16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32, gc_highcheckers: u32) -> Result<(), Box<dyn Error>> {
    let root = xconnection.setup().roots[0].root;
    let mut windows = Vec::new();
    
    if let Ok(root_tree) = xconnection.query_tree(root)?.reply() {

        for &target in &root_tree.children {
            if target == window || target == root {
                continue;
            }
            if let Ok(tree) = xconnection.query_tree(target)?.reply() {
                if !tree.children.is_empty() {
                    let client = tree.children.last().copied().unwrap_or(target);
                    if let Some(client_title) = grabwindowtitle(xconnection, client)? {
                        windows.push((target, client_title, client));
                        continue;
                    }
                }
            }
            if let Ok(attrs) = xconnection.get_window_attributes(target)?.reply() {
                if attrs.map_state != MapState::VIEWABLE {
                    continue;
                }
                if let Some(title) = grabwindowtitle(xconnection, target)? {
                    windows.push((target, title, target));
                }
            }
        }
    }

    if windows.is_empty() {
        return Ok(());
    }
    //workingwidth
    let targetwidth = 160;
    let windowscount = windows.len() as i16;
    let requiredwidth = (targetwidth * windowscount as i16) + (3 * (windowscount - 1) as i16);
	let finalwidth = if requiredwidth > workingwidth {
		((workingwidth - (3 * (windowscount as i16 - 1))) / windowscount as i16).max(30)
	} else {
		targetwidth
	};

    let mut focused_window = None;
    if let Ok(root_tree) = xconnection.query_tree(root)?.reply() {
        for &w in root_tree.children.iter().rev() {
            if w != window {
                focused_window = Some(w);
                break;
            }
        }
    }

    let mut offset = startx;
	
    for (i, (frame_id, title, client_id)) in windows.iter().enumerate() {
        


        let max_chars = ((finalwidth - 20) / 6).max(1);
        let display_title = if title.len() > max_chars as usize {
            if max_chars > 3 {
                format!("{}...", &title[0..(max_chars - 3) as usize])
            } else {
                title[0..max_chars as usize].to_string()
            }
        } else {
            title.clone()
        };
		if Some(*frame_id) == focused_window {
			drawdepressedbumpyframe(xconnection, window, offset, 4, finalwidth, 21, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_highcheckers)?;
			drawpng(&xconnection, window, "computer.png", offset + 4, 8, 16, 16, COLOURS[HIGHBACKGROUND_COLOUR])?;
			if !display_title.is_empty() {
				xconnection.image_text8(window, gc_lowlight, offset + 22, 20, display_title.as_bytes())?;
			}
		} else {
			drawbumpyframe(xconnection, window, offset, 4, finalwidth, 21, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground)?;
			drawpng(&xconnection, window, "computer.png", offset + 4, 7, 16, 16, COLOURS[HIGHBACKGROUND_COLOUR])?;
			if !display_title.is_empty() {
				xconnection.image_text8(window, gc_lowlight, offset + 22, 19, display_title.as_bytes())?;
			}
		}
		


        offset += finalwidth + 3;
        if offset + finalwidth > startx + workingwidth {
            println!("Uh-oh! We are out of space and Window {}!", i+1);
            break;
        }
    }
    
    xconnection.flush()?;
    Ok(())
}