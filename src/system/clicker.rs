use std::{
    error::Error,
    fs::File,
    io::{self, Read, BufReader},
    thread,
    time::{SystemTime, Duration},
};
use std::sync::{Mutex, OnceLock};
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
use crate::trundle::drawbookerframe;
use crate::trundle::drawradiobutton;
use crate::trundle::drawcheckbox;
use crate::trundle::drawsystemframe;
use crate::drawdepressedbumpyframe;
use crate::drawdepressedframe;
use crate::insertpanelwindow;
use crate::focuswindow;
use crate::drawpng;
use crate::drawpngcover;
use crate::drawbumpyframe;

use crate::createwindow;
use crate::createframelesswindow;
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





pub fn startprogram(xconnection: &impl Connection, screen: &Screen, panel: Window, clickmenuitems: &mut [[String; 3]; 16], clickmenusize: &mut u8, gc_highlight: Gcontext, gc_lowlight: Gcontext, gc_highbackground: Gcontext, gc_lowbackground: Gcontext, gc_titlebar: Gcontext, gc_titlebartext: Gcontext, wm: &mut WindowManager) -> Window { 

    let mut clickerheight: i16 = 0;

	//Loop through clickmenuitems to determine the size of height.
	//No Divider = 20, Dividier = 6
    for i in 0..(*clickmenusize as usize) {
        let label = &clickmenuitems[i][0];
        if label == "Divider" {
            clickerheight += 6;
        } else {
            clickerheight += 20;
        }
    }

	const WIDTH: i16 = 180;
	const STARTX: i16 = WIDTH;
	const STARTY: i16 = 0;
	

	match createframelesswindow(xconnection, screen, 50, 50, WIDTH as u16 + 1, clickerheight as u16 + 1, b"Clicker", 500, 500, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext, wm) {
		Ok(clicker) => {
			//Draw menu...
			
			//drawdepressedbumpyframe(&xconnection, clicker, 0, 0, 180, clickerheight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_highlight, gc_highbackground);
			
			drawsystemframe(&xconnection, clicker, STARTX, STARTY, WIDTH, clickerheight, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground);
			//drawdepressedframe(&xconnection, clicker, STARTX, STARTY, WIDTH, clickerheight, gc_lowlight, gc_highbackground);
			//drawdepressedframe(&xconnection, clicker, STARTX - 1, STARTY + 1, WIDTH - 2, clickerheight - 2, gc_lowbackground, gc_highlight);
			//xconnection.poly_fill_rectangle(clicker, gc_highbackground, &[Rectangle { x: 2, y: 2, width: WIDTH as u16 - 3, height: clickerheight as u16 - 3}]); //Draw panel background.
			
			//drawdepressedframe(&xconnection, clicker, STARTX - 1, STARTY + 1, WIDTH - 2, clickerheight-6, gc_highlight, gc_lowbackground);
			
			
			match focuswindow(wm, xconnection, panel, clicker, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext) {
				Ok(_) => clicker,
				Err(_) => 0,
			}
		},
		Err(_) => 0,
	}
}

pub fn drawclickmenu<C: Connection>(xconnection: &C, window: u32, startx: i16, starty: i16, clickmenuitems: &mut [[String; 3]; 16], clickmenusize: &mut u8, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32) -> Result<(), Box<dyn Error>> {

	//drawbumpyframe(&xconnection, window, startx, starty, 300, height, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground)?;


    Ok(())


}
