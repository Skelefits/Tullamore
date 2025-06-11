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
use crate::drawdepressedbumpyframe;
use crate::insertpanelwindow;
use crate::focuswindow;
use crate::drawpng;
use crate::drawpngcover;
use crate::drawbumpyframe;
use crate::createwindow;
use crate::WindowManager;

static GAME: OnceLock<Mutex<GameState>> = OnceLock::new();

use crate::trundle::{
    COLOURS,
    HIGHBACKGROUND_COLOUR,
    LOWBACKGROUND_COLOUR,
    HIGHLIGHT_COLOUR,
    LOWLIGHT_COLOUR,
    WALLPAPER_COLOUR,
    TITLEBAR_COLOUR
};

struct GameState {
    //States
    strength: u8,
    agility: u8,
    speed: u8,
    health: u8,
    sense: u8,
    charm: u8,
    orb: u8,
    
    //Progress
    location: u8,
    mode: u8,
    points: u8,
    items: u8,
	event: u8,
	step: u8,
	time: u8,
    
    //Combat
    currenthealth: u8,
}

impl GameState {
    fn new() -> Self {
        Self {
            strength: 0,
            agility: 0,
            speed: 0,
            health: 0,
            sense: 0,
            charm: 0,
            orb: 0,

            location: 0,
            mode: 0,
            points: 0,
            items: 0,
            event: 0,
            step: 0,
            time: 0,


            currenthealth: 0,
        }
    }

    fn clear(&mut self) {
        *self = Self::new();
    }
}

pub fn startprogram(xconnection: &impl Connection, screen: &Screen, panel: Window, width: i16, height: i16, panelindex: &mut [u8; 6], panelitems: &mut [[u8; 1]; 128], panelcoordinates: &mut [[i16; 2]; 128], panelwindows: &mut [[u32; 1]; 128], panelicons: &mut [[String; 4]; 32], gc_highlight: Gcontext, gc_lowlight: Gcontext, gc_highbackground: Gcontext, gc_lowbackground: Gcontext, gc_titlebar: Gcontext, gc_titlebartext: Gcontext, wm: &mut WindowManager) -> Window { 
	let inputwidth = 900;
	let inputheight = 800;

	let widthscale = inputwidth as f32 / 620.0;
	let heightscale = inputheight as f32 / 428.0;
	let scale = widthscale.min(heightscale);

    let buttonwidth = (80.0 * scale).round() as i16;
    let buttonheight = (65.0 * scale).round() as i16;
	
	let gamewidth = (620.0 * scale).round() as u16;
	
	let gameheight = 3 + (6 * (buttonheight + 5)) as u16;
	
	//let gameheight = (428.0 * scale).round() as u16;
	
	
	
	

	match createwindow(xconnection, screen, 50, 50, gamewidth, gameheight, b"Superbun", width, height, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext, wm) {
		Ok(superbun) => {
			insertpanelwindow(panelindex, superbun,  panelitems, panelcoordinates, panelwindows, panelicons);
			basicscreen(xconnection, superbun, gamewidth, gameheight, buttonwidth, buttonheight, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground);
			match focuswindow(wm, xconnection, panel, superbun, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground, gc_titlebar, gc_titlebartext) {
				Ok(_) => superbun,
				Err(_) => 0,
			}
		},
		Err(_) => 0,
	}
}

fn newgame() {
	let game = GAME.get_or_init(|| Mutex::new(GameState::new()));
	game.lock().unwrap().clear();
}

pub fn endgame() {
    if let Some(game) = GAME.get() {
        game.lock().unwrap().clear();
    }
}

pub fn basicscreen(xconnection: &impl Connection, superbun: Window, gamewidth: u16, gameheight: u16, buttonwidth: i16, buttonheight: i16, gc_highlight: Gcontext, gc_lowlight: Gcontext, gc_highbackground: Gcontext, gc_lowbackground: Gcontext) {

	

	//Draw the background colour.
	xconnection.poly_fill_rectangle(superbun, gc_highbackground, &[Rectangle { x: 0, y: 0, width: gamewidth, height: gameheight }]);


	//let width = 80;
	//let height = 65;

	let startx = 8 + buttonwidth;
	let middlewidth = gamewidth as i16 - 16 - (buttonwidth * 2);


	

	


	//drawradiobutton(xconnection, superbun, 10, 10, 12, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground);

	//drawcheckbox(xconnection, superbun, 30, 30, 12, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground);


	

	let text = vec![
		"The Dark Forest".to_string(),
		"You find yourself in a dark forest. The day is dark and the sun is blue. There are too many purple cows. H H H Hi. Hi. Hi.".to_string(), // Description
		"Attack the dragon! There are too many purple cows. What are you looking at?".to_string(),
		"The day is dark and the sun is blue. There are too many purple cows. Hack me and you. Obviously.".to_string(),
		"Try to sneak past".to_string(),
		"Cast a spell".to_string(),
		"Run Away!".to_string(),
		"Hidden Option".to_string(),
	];

	let flair = vec![1, 0, 2, 0, 2, 2, 2, 2]; 

	let mut y = 0 as i16;
	
	//Draw the below in four rows of two.
    for row in 0..6 {
        y = 4 + (row * (buttonheight + 5));
		drawbookerframe(xconnection, superbun, 4, y, buttonwidth, buttonheight, 3, gc_highlight, gc_lowlight, 0, gc_lowbackground);
		
		drawbookerframe(xconnection, superbun, gamewidth as i16 - 4 - buttonwidth, y, buttonwidth, buttonheight, 3, gc_highlight, gc_lowlight, 0, gc_lowbackground);
    }
	
	y = y + buttonheight - 137;

	
	
	
	//Draw chatbox
	drawbookerframe(xconnection, superbun, startx, y, middlewidth, 138, 1, gc_lowbackground, 0, 0, gc_highlight);
	
	drawtextbox(xconnection, superbun, startx + 4, y, middlewidth - 6, 148, &text, &flair, 6, 16, gc_lowlight, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground);


	//The default size is 280 height for the image box.
	
	y = y - 8;

	//Draw image container
	drawpngcover(&xconnection, superbun, "computer.png", startx, 3, middlewidth as u16, y as u16, COLOURS[HIGHBACKGROUND_COLOUR]);
	
	drawbookerframe(xconnection, superbun, startx, 3, middlewidth, y, 3, gc_lowbackground, 0, 0, gc_highlight);

}

fn gameevent(xconnection: &impl Connection, superbun: Window, gc_highlight: Gcontext, gc_lowlight: Gcontext, gc_highbackground: Gcontext, gc_lowbackground: Gcontext) {


}

pub fn drawtextbox<C: Connection>(xconnection: &C, window: u32, startx: i16, starty: i16, width: i16, height: i16, text: &[String], flair: &[u8], charwidth: i16, lineheight: i16, gc_text: u32, gc_highlight: u32, gc_lowlight: u32, gc_highbackground: u32, gc_lowbackground: u32) -> Result<(), Box<dyn Error>> {
    
    let mut y = starty + lineheight;
    
    for (i, line_text) in text.iter().enumerate() {
        if y > starty + height - lineheight {
            break; // Don't draw beyond textbox bounds
        }
        
        let line_flair = flair.get(i).unwrap_or(&0);
        
        match *line_flair {
            0 => { //Align Left Text
                let wrapped_lines = wraptext(line_text, width, charwidth);
                for wrapped_line in wrapped_lines {
                    if y > starty + height - lineheight {
                        break;
                    }
                    xconnection.image_text8(window, gc_text, startx, y, wrapped_line.as_bytes())?;
                    y += lineheight;
                }
            }
            1 => { //Centered Text
                let wrapped_lines = wraptext(line_text, width, charwidth);
                for wrapped_line in wrapped_lines {
                    if y > starty + height - lineheight {
                        break;
                    }
                    let text_width = (wrapped_line.len() as i16) * charwidth;
                    let centered_x = startx + (width - text_width) / 2;
                    xconnection.image_text8(window, gc_text, centered_x, y, wrapped_line.as_bytes())?;
                    y += lineheight;
                }
            }
            2 => { //Radio Button
                drawradiobutton(xconnection, window, startx, y - 10, 12, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground)?;
                let truncated_text = squishtext(line_text, width - 16, charwidth);
                xconnection.image_text8(window, gc_text, startx + 16, y, truncated_text.as_bytes())?;
                y += lineheight;
            }
            3 => { //Checkbox
                drawcheckbox(xconnection, window, startx, y - 10, 12, gc_highlight, gc_lowlight, gc_highbackground, gc_lowbackground)?;
                let truncated_text = squishtext(line_text, width - 16, charwidth);
                xconnection.image_text8(window, gc_text, startx + 16, y, truncated_text.as_bytes())?;
                y += lineheight;
            }
            4..=255 => { //PNGs
                let wrapped_lines = wraptext(line_text, width, charwidth);
                for wrapped_line in wrapped_lines {
                    if y > starty + height - lineheight {
                        break;
                    }
                    xconnection.image_text8(window, gc_text, startx, y, wrapped_line.as_bytes())?;
                    y += lineheight;
                }
            }
        }
    }
    
    Ok(())
}

fn wraptext(text: &str, width: i16, charwidth: i16) -> Vec<String> {
    let max_chars_per_line = (width / charwidth) as usize;
    let mut lines = Vec::new();
    let words: Vec<&str> = text.split_whitespace().collect();
    
    if words.is_empty() {
        return lines;
    }
    
    let mut current_line = String::new();
    
    for word in words {
        // Check if adding this word would exceed the line length
        let test_line = if current_line.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current_line, word)
        };
        
        if test_line.len() <= max_chars_per_line {
            current_line = test_line;
        } else {
            // Start a new line
            if !current_line.is_empty() {
                lines.push(current_line);
            }
            
            // Handle words that are too long for a single line
            if word.len() > max_chars_per_line {
                let mut remaining = word;
                while remaining.len() > max_chars_per_line {
                    lines.push(remaining[0..max_chars_per_line].to_string());
                    remaining = &remaining[max_chars_per_line..];
                }
                current_line = remaining.to_string();
            } else {
                current_line = word.to_string();
            }
        }
    }
    
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    
    lines
}