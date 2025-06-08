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
use crate::trundle::drawbookerframe;
use crate::trundle::drawradiobutton;
use crate::drawdepressedbumpyframe;
use crate::insertpanelwindow;
use crate::focuswindow;
use crate::drawpng;
use crate::drawbumpyframe;
use crate::createwindow;
use crate::WindowManager;


pub fn startprogram(xconnection: &impl Connection, screen: &Screen, panel: Window, width: i16, height: i16, panelindex: &mut [u8; 6], panelitems: &mut [[u8; 1]; 128], panelcoordinates: &mut [[i16; 2]; 128], panelwindows: &mut [[u32; 1]; 128], panelicons: &mut [[String; 4]; 32], gc_highlight: Gcontext, gc_lowlight: Gcontext, gc_highbackground: Gcontext, gc_lowbackground: Gcontext, gc_titlebar: Gcontext, gc_titlebartext: Gcontext, wm: &mut WindowManager) -> u8 { 
	match createwindow(xconnection, screen, 50, 50, 620, 430, b"Superbun", width, height, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext, wm) {
		Ok(booker) => {
			insertpanelwindow(panelindex, booker, panelitems, panelcoordinates, panelwindows, panelicons);
			basicscreen(xconnection, booker, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground);
			match focuswindow(wm, xconnection, panel, booker, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext) {
				Ok(_) => 40,
				Err(_) => 0,
			}
		},
		Err(_) => 0,
	}
}





pub fn basicscreen(xconnection: &impl Connection, superbun: Window, gc_highlight: Gcontext, gc_lowlight: Gcontext, gc_highbackground: Gcontext, gc_lowbackground: Gcontext) {
	//Draw the background colour.
	xconnection.poly_fill_rectangle(superbun, gc_highbackground, &[Rectangle { x: 0, y: 0, width: 620, height: 430 }]);

	let width = 80;
	let height = 65;

	let startx = 8 + width;
	let middlewidth = 620 - 16 - (width * 2);

	//Draw image container
	drawbookerframe(xconnection, superbun, startx, 4, middlewidth, 280, 3, gc_lowbackground, 0, 0, gc_highlight);
	
	//Draw location info
	drawbookerframe(xconnection, superbun, startx, 288, middlewidth, 138, 1, gc_lowbackground, 0, 0, gc_highlight);
	


drawradiobutton(xconnection, superbun, 10, 10, 12, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground);



	
	//Draw the below in four rows of two.
    for row in 0..6 {
        let y = 4 + (row * (height + 6));
		drawbookerframe(xconnection, superbun, 4, y, width, height, 3, gc_highlight, gc_lowlight, 0, gc_lowbackground);
		
		drawbookerframe(xconnection, superbun, 620 - 4 - width, y, width, height, 3, gc_highlight, gc_lowlight, 0, gc_lowbackground);
    }
}
