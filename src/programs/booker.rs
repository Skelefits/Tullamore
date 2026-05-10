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
use crate::drawdepressedbumpyframe;
use crate::insertpanelwindow;
use crate::focuswindow;
use crate::drawpng;
use crate::drawbumpyframe;
use crate::createwindow;
use crate::WindowManager;


pub fn startprogram(xconnection: &impl Connection, screen: &Screen, panel: Window, width: i16, height: i16, panelindex: &mut [u8; 6], panelitems: &mut [[u8; 1]; 128], panelcoordinates: &mut [[i16; 2]; 128], panelwindows: &mut [[u32; 1]; 128], panelicons: &mut [[String; 4]; 32], gc_highlight: Gcontext, gc_lowlight: Gcontext, gc_highbackground: Gcontext, gc_lowbackground: Gcontext, gc_titlebar: Gcontext, gc_titlebartext: Gcontext, wm: &mut WindowManager, poly_lowlight: &mut Vec<Segment>, poly_index: &mut Vec<u8>, poly_windoworcolour: &mut Vec<u32>) -> u8 { 
	match createwindow(xconnection, screen, 50, 50, 560, 340, b"Booker", width, height, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext, wm) {
		Ok(booker) => {
			insertpanelwindow(panelindex, booker, panelitems, panelcoordinates, panelwindows, panelicons);
			basicscreen(xconnection, booker, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground);
			match focuswindow(wm, xconnection, panel, booker, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext, poly_lowlight, poly_index, poly_windoworcolour) {
				Ok(_) => 40,
				Err(_) => 0,
			}
		},
		Err(_) => 0,
	}
}





pub fn basicscreen(xconnection: &impl Connection, booker: Window, gc_highlight: Gcontext, gc_lowlight: Gcontext, gc_highbackground: Gcontext, gc_lowbackground: Gcontext) {
	//Draw the background colour.
	xconnection.poly_fill_rectangle(booker, gc_highbackground, &[Rectangle { x: 0, y: 0, width: 560, height: 340 }]);

	
	//Draw chapter info.
	drawbookerframe(xconnection, booker, 8, 5, 182, 18, 1, gc_lowbackground, 0, 0, gc_highlight);
	
	//Draw left container.
	drawbookerframe(xconnection, booker, 8, 26, 182, 310, 3, gc_lowbackground, 0, 0, gc_highlight);
	
	
	
	

	let width = 80;
	let height = 65;
	
	//Draw the below in four rows of two.
    for row in 0..4 {
        for col in 0..2 {
            let x = 16 + (col * (width + 6));
            let y = 50 + (row * (height + 6));
			drawbookerframe(xconnection, booker, x, y, width, height, 3, gc_highlight, gc_lowlight, 0, gc_lowbackground);
        }
    }
}
