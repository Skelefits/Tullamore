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
use crate::trundle::drawclickbutton;
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


const DIVIDER: i16 = 8;
const ITEM: i16 = 20;
const SEARCH: i16 = 30;
const OFFSET: i16 = 3;

pub fn startprogram(xconnection: &impl Connection, screen: &Screen, panel: Window, clickmenuitems: &[[String; 3]; 16], clickmenusize: &u8, screenwidth: &i16, screenheight: &i16, gc_highlight: Gcontext, gc_lowlight: Gcontext, gc_highbackground: Gcontext, gc_lowbackground: Gcontext, gc_titlebar: Gcontext, gc_titlebartext: Gcontext, wm: &mut WindowManager) -> Window { 

    let mut clickerheight: i16 = OFFSET + OFFSET;



	//Loop through clickmenuitems to determine the size of height.
	//No Divider = 20, Dividier = 6
    for i in 0..(*clickmenusize as usize) {
        let label = &clickmenuitems[i][0];
        if label == "Divider" {
            clickerheight += DIVIDER;
		} else if label == "Search" {
			clickerheight += SEARCH;
        } else {
            clickerheight += ITEM;
        }
    }

	const WIDTH: i16 = 160;
	const STARTX: i16 = WIDTH;
	const STARTY: i16 = 0;

	match createframelesswindow(xconnection, screen, 0, screenheight - clickerheight - 29, WIDTH as u16 + 1, clickerheight as u16 + 1, b"Clicker", 500, 500, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext, wm) {
		Ok(clicker) => {
			//Draw menu...
			

			drawclickmenu(&xconnection, clicker, clickmenuitems, clickmenusize, STARTX, STARTY, WIDTH, clickerheight, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext);
			
			
			match focuswindow(wm, xconnection, panel, clicker, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext) {
				Ok(_) => clicker,
				Err(_) => 0,
			}
		},
		Err(_) => 0,
	}
}

fn drawclickmenu<C: Connection>(xconnection: &C, clicker: u32, clickmenuitems: &[[String; 3]; 16], clickmenusize: &u8, startx: i16, starty: i16, clickerwidth: i16, clickerheight: i16, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32, gc_titlebar: u32, gc_titlebartext: u32) {
	
	//Draw border and background.
	drawsystemframe(&xconnection, clicker, startx, starty, clickerwidth, clickerheight, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground);
	//Draw OS Text Background
	xconnection.poly_fill_rectangle(clicker, gc_titlebar, &[Rectangle { x: OFFSET, y: OFFSET, width: 21, height: (clickerheight - OFFSET - OFFSET + 1) as u16}]);
	//Draw OS Text Here
	//???
	//Loop through all menu items and draw them to the screen.
	

	
//const DIVIDER: i16 = 8;
//const ITEM: i16 = 20;
//const SEARCH: i16 = 30;
	
	let mut loopy = clickerheight;
	
	if &clickmenuitems[0][0] == "Search" {
		loopy = loopy - SEARCH + 7
	} else {
		loopy = loopy - ITEM + 7
	}
	
	const STARTLOGO: i16 = 27;
	const HALFDIVIDER: i16 = DIVIDER / 2;
	
    for i in 0..(*clickmenusize as usize) {
		let label = &clickmenuitems[i][0];
        if label == "Divider" {
			//TODO: Make divider, make it a standard non-poly line.
			xconnection.poly_line(CoordMode::PREVIOUS, clicker, gc_lowbackground, &[
				Point { x: STARTLOGO, y: loopy + HALFDIVIDER + 1 },
				Point { x: clickerwidth - STARTLOGO - 5, y: 0 },
			]);
			xconnection.poly_line(CoordMode::PREVIOUS, clicker, gc_highlight, &[
				Point { x: STARTLOGO, y: loopy + HALFDIVIDER + 2 },
				Point { x: clickerwidth - STARTLOGO - 5, y: 0 },
			]);
			
            loopy = loopy - DIVIDER;
		} else if label == "Search" {
			loopy = loopy - SEARCH;
        } else {
			drawpng(&xconnection, clicker, "computer.png", STARTLOGO, loopy - 8, 16, 16, COLOURS[HIGHBACKGROUND_COLOUR]);
            xconnection.image_text8(clicker, gc_lowlight, 48, loopy + 4, label.as_bytes());
			loopy = loopy - ITEM;
        }
    }
}

pub fn endprogram<C: Connection>(wm: &mut WindowManager, xconnection: &C, window: u32, system: Window, panelcoordinates: [[i16; 2]; 128], gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32) -> Result<(), Box<dyn Error>> {
	drawclickbutton(&xconnection, window, panelcoordinates[0][0], 4, panelcoordinates[0][1], 21, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground)?;
	wm.removewindow(&xconnection, 0, system);
	//system = 0;
    Ok(())
}