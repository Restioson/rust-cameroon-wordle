use chrono::{Local, NaiveDate, TimeZone};
use gloo::events::EventListener;
use rand::prelude::*;
use std::fmt;
use std::fmt::{Display, Formatter, Write};
use std::iter;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use yew::{function_component, html, AttrValue, Callback, Component, Context, Html, Properties};

const SEED: u64 = 11530789889988543623;
const WORD_LENGTH: usize = 6;
const GUESSES: usize = 6;

/// This is our main function - we just initialise logging and panic output
#[wasm_bindgen(start)]
fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
}

/// This wil run our wordle game
#[wasm_bindgen]
pub async fn start_wordle() {
    let wrap = gloo::utils::document()
        .get_element_by_id("main_wrap")
        .unwrap();
    yew::Renderer::<Game>::with_root(wrap).render();
}

/// This is a very simple example of how we can call our own JS function
#[wasm_bindgen(module = "/wordle.js")]
extern "C" {
    fn share(text: String);
}

enum GameMessage {
    Key(char),
    CloseModal,
    Share,
}

struct Game {
    /// All possible words that can be guessed
    dictionary: Vec<String>,

    /// The current word that we are trying to guess
    word_to_guess: String,

    /// A list of every guess the user has made. While the game is active, the current guess (the
    /// last one in the list) is being modified to record their keystrokes as they type
    guesses: Vec<Guess>,

    /// Whether the game is in progress, won, or lost
    state: GameState,

    /// Our visual keyboard at the bottom of the screen
    keyboard: Vec<Vec<Key>>,

    /// This lets us listen to the user's keystrokes
    kbd_listener: Option<EventListener>,

    /// Which # wordle we are on (see create function)
    nth_wordle: u32,

    /// Some state information about the game's UI
    modal_open: bool,
    shared: bool,
}

impl Game {
    /// Evaluate how correct the current guess is. Returns None if the current word that the user
    /// has entered is not a valid word in our dictionary. Otherwise, returns a list which indicates
    /// whether each letter in the guess was:
    /// - Green - correct & in the right place
    /// - Yellow - correct, but in the wrong place
    /// - Grey - incorrect and not in the word at all
    fn evaluate_guess(&self, guess: &Guess) -> Option<[CharGuessResult; WORD_LENGTH]> {
        // Convert the guess to a string of its characters
        let guess = &guess.letters;
        let guess_str: String = guess.iter().map(|l| l.letter).collect();

        // Oops - this word does not exist, so cancel the user's guess
        if !self.dictionary.iter().any(|word| *word == guess_str) {
            return None;
        }

        // Let's figure out how many letters they got correct
        // We start with assuming every letter is correct, and we update this as we find correct
        // letters in their guess
        let mut result = [CharGuessResult::Incorrect; WORD_LENGTH];

        // First, we will check if they have any letters in the right place in the word.
        // For instance, the guess APPLE for the word ABOUT has the A in the right place (green)
        //
        // We want to keep track of which of their letters were _not_ in the word to guess, to see
        // which letters we need to grey out, so we will remove correct letters from that list as we
        // go
        let mut remaining_letters = self.word_to_guess.clone();
        let mut removal_cursor = 0;
        for idx in 0..WORD_LENGTH {
            if self.word_to_guess.chars().nth(idx).unwrap() == guess[idx].letter {
                // This letter is correct! Remove it from the list of remaining letters
                result[idx] = CharGuessResult::Correct;
                let _ = remaining_letters.remove(removal_cursor);
            } else {
                // This letter isn't in the word in the same position
                removal_cursor += 1;
            }
        }

        // Now, we check which letters they guessed that were still _in_ the word, just in the
        // wrong place (yellow)
        //
        // We do this by going through all the letters that they got in the wrong place and then
        // checking if any of these were actually in the word, just somewhere else
        for (idx, res) in result
            .iter_mut()
            .enumerate()
            .filter(|(_, res)| **res == CharGuessResult::Incorrect)
        {
            if let Some(idx) = remaining_letters.as_str().find(guess[idx].letter) {
                *res = CharGuessResult::WrongPlace;
                let _ = remaining_letters.remove(idx);
            }
        }

        // Return how right the guess was!
        Some(result)
    }

    fn guess(&mut self) {
        // The game is over, so we ignore them trying to guess this
        if self.state != GameState::Continue {
            return;
        }

        // See if their guess is a valid word - if not (None), we exit the function
        let result = match self.evaluate_guess(self.current_guess()) {
            Some(r) => r,
            None => return,
        };

        // Guess is valid, so lets update the letters on the keyboard to show which are correct and
        // in the right place (green), correct but in the wrong place (yellow), and incorrect and
        // not in the word at all (grey)
        let guess = self.guesses.last_mut().unwrap();
        for (letter, state) in guess.letters.iter_mut().zip(result.iter()) {
            letter.state = Some(*state);

            for row in &mut self.keyboard {
                for key in row {
                    if key.key == letter.letter && key.state.map(|s| s < *state).unwrap_or(true) {
                        key.state = Some(*state);
                    }
                }
            }
        }

        // If all the letters were in the right place, the user has won!
        if result.iter().all(|r| *r == CharGuessResult::Correct) {
            self.state = GameState::Won;
            self.modal_open = true;
            return;
        }

        self.state = if self.guesses.len() == GUESSES {
            // Otherwise, we've lost the game if they've made their last guess
            self.modal_open = true;
            GameState::Lost
        } else {
            // ... or we keep going, putting a new blank guess into our list
            self.guesses.push(Guess::default());
            GameState::Continue
        };
    }

    fn current_guess(&self) -> &Guess {
        self.guesses.last().unwrap()
    }

    fn current_guess_mut(&mut self) -> &mut Guess {
        self.guesses.last_mut().unwrap()
    }
}

// We use this Display impl to format our game for sharing
impl Display for Game {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let guesses = self.guesses.len();
        let score = if self.state == GameState::Won {
            guesses.to_string()
        } else {
            "X".to_string()
        };

        let nth_wordle = self.nth_wordle + 1;
        writeln!(f, "Rust Cameroon Wordle {nth_wordle}  {score} / {GUESSES}")?;

        for guess in &self.guesses {
            let result = match self.evaluate_guess(guess) {
                Some(r) => r,
                None => continue,
            };

            for char in result {
                char.fmt(f)?;
            }

            f.write_char('\n')?;
        }

        Ok(())
    }
}

// This tells Yew how to create and render our game to HTML
impl Component for Game {
    type Message = GameMessage;
    type Properties = ();

    /// `create` creates our component struct.
    /// Here we initialise the game - we load the dictionary and pick the starting word
    fn create(_ctx: &Context<Self>) -> Self {
        let mut dictionary: Vec<_> = include_str!("words.txt")
            .lines()
            .filter(|word| word.trim().len() == WORD_LENGTH)
            .map(|word| word.to_uppercase())
            .collect();

        log::info!("{} total words in dictionary", dictionary.len());

        // Dedup only checks for adjacent duplicates, so we must sort first
        dictionary.sort_unstable();
        dictionary.dedup();

        // Figure out what "number" of Wordle we are on - i.e, # of days since the start
        let start_date = Local
            .from_local_datetime(&NaiveDate::from_ymd_opt(2025, 2, 10).unwrap().into())
            .unwrap();
        let now = Local::now();
        let nth_wordle = (now - start_date).num_days() as usize;

        // We also want to carry on if we run out of words, so we calculate how many times we have
        // already completed a full cycle, and shuffle it _again_ for each time to get a unique
        // order of words to guess each cycle
        let cycle = nth_wordle / dictionary.len();
        let nth_wordle = nth_wordle % dictionary.len();

        // We also want to get an initial shuffle for the first cycle, so we use cycle + 1
        let mut rng = StdRng::seed_from_u64(SEED);
        for _ in 0..cycle + 1 {
            dictionary.shuffle(&mut rng);
        }

        log::info!("Today is the {}th wordle (cycle {})", nth_wordle, cycle);

        // Pick today's wordle from the shuffled dictionary
        let word = dictionary[dictionary.len() - 1 - nth_wordle].clone();

        // Create the keyboard
        let rows = ["QWERTYUIOP", "|ASDFGHJKL|", "\nZXCVBNM\x08"];
        let keys: Vec<Vec<Key>> = rows
            .into_iter()
            .map(|row| row.chars().map(Key::from).collect())
            .collect();

        // Create our initial guess - this will be blank, and the user adds to it by typing
        let guesses = vec![Guess::default()];

        Game {
            dictionary,
            word_to_guess: word,
            guesses,
            state: GameState::Continue,
            keyboard: keys,
            kbd_listener: None,
            nth_wordle: nth_wordle as u32,
            modal_open: false,
            shared: false,
        }
    }

    /// This runs whenever we sent a 'message' to our game. This happens in reponse to the user
    /// doing something, like pressing one of our on-screen keyboard buttons, or clicking on the
    /// Share button
    fn update(&mut self, _ctx: &Context<Self>, msg: GameMessage) -> bool {
        match msg {
            GameMessage::CloseModal => {
                self.modal_open = false;
                true
            }
            GameMessage::Share => {
                share(self.to_string());
                self.shared = true;
                true
            }
            GameMessage::Key(key) => {
                if self.state != GameState::Continue {
                    return false;
                }

                match key {
                    // Handles backspace by removing the current letter
                    '\x08' => self.current_guess_mut().letters.pop().is_some(),
                    // An Enter causes the user to guess
                    '\n' => {
                        if self.current_guess_mut().letters.len() == WORD_LENGTH {
                            self.guess();
                            true
                        } else {
                            false
                        }
                    }
                    // Otherwise, we just add the letter to their guess (if there is space)
                    _ => {
                        let guess = &mut self.current_guess_mut().letters;

                        if guess.len() < WORD_LENGTH {
                            guess.push(key.to_ascii_uppercase().into());
                            true
                        } else {
                            false
                        }
                    }
                }
            }
        }
    }

    /// Render our game as HTML
    fn view(&self, ctx: &Context<Self>) -> Html {
        // Get a list of every guess that the user has made, padded with empty guesses if the
        // game is still in progress
        let guesses_padded = self
            .guesses
            .iter()
            .cloned()
            .chain(iter::repeat(Guess::default()))
            .take(GUESSES);

        // Turn the list of guesses into HTML
        let guesses = guesses_padded
            .map(|guess| {
                // If one of the guesses is incomplete, just pad it with empty letters
                let letters_padded = guess
                    .letters
                    .iter()
                    .copied()
                    .chain(iter::repeat(GuessLetter::default()))
                    .take(WORD_LENGTH);

                // Create a div for each letter
                let letters = letters_padded
                    .map(|letter| {
                        // The style attribute ensures the colour reflects how correct the guess was
                        html! {
                            <div class="guess_letter" style={ letter.css() }>
                                { letter.letter }
                            </div>
                        }
                    })
                    .collect::<Html>();

                // Wrap all the letters in a row div
                html! {
                    <div class="guess">{ letters }</div>
                }
            })
            .collect::<Html>();

        let victory_or_defeat_message = if self.state == GameState::Won {
            "Congratulations! A new wordle will be available tomorrow."
        } else {
            "Unlucky... try again tomorrow."
        };

        let modal_classes = if self.modal_open {
            "modal open"
        } else {
            "modal"
        };

        let shared = if self.shared { "Copied!" } else { "" };

        // These are our event listeners which send messages to our update() method
        let on_key = ctx.link().callback(GameMessage::Key);
        let on_share = ctx.link().callback(|_| GameMessage::Share);
        let on_close = ctx.link().callback(|_| GameMessage::CloseModal);

        // We include our icons as raw SVGs, which are luckily valid as HTML tags
        let close_icon = Html::from_html_unchecked(AttrValue::from(include_str!("close_icon.svg")));
        let share_icon = Html::from_html_unchecked(AttrValue::from(include_str!("share_icon.svg")));

        html! {
            <>
                <h1>{ "Rust Cameroon Wordle" }</h1>

                <div id="guesses_wrapper">
                    <div id="guesses">{ guesses }</div>
                </div>

                <div id="keyboard">
                    <Keyboard keys={ self.keyboard.clone() } on_click={ on_key }/>
                </div>

                <div id="wordle_modal" class={ modal_classes }>
                    <div class="column_list">

                        <button id="close_modal" onclick={ on_close }>
                            { close_icon }
                        </button>
                        <p id="result">{ victory_or_defeat_message }</p>

                        <p id="todays_word">
                            { "Today's word was " }
                            { self.word_to_guess.to_lowercase() }
                            { "." }
                        </p>

                        <button id="share" onclick={ on_share }>
                            <span>{ "Share with your friends" }</span>
                            { share_icon }
                        </button>
                        <p id="shared">{ shared }</p>
                    </div>
                </div>
            </>
        }
    }

    /// This is called after we are rendered - we use this to register a key press callback
    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        // We only need to register the callback on first render
        if !first_render {
            return;
        }

        // Register our callback
        let document = gloo::utils::document();

        let callback = ctx.link().callback(|c: char| GameMessage::Key(c));
        let listener = EventListener::new(&document, "keydown", move |event| {
            let event = event.dyn_ref::<web_sys::KeyboardEvent>().unwrap();

            let first_char = event.key().chars().next().unwrap();

            let msg = match &event.key() as &str {
                "Backspace" => '\x08',
                "Enter" => '\n',
                _ if event.key().len() == 1 && first_char.is_ascii_alphabetic() => first_char,
                _ => return,
            };

            callback.emit(msg);
        });

        self.kbd_listener.replace(listener);
    }
}

#[derive(Properties, PartialEq)]
struct KeyboardProps {
    keys: Vec<Vec<Key>>,
    on_click: Callback<char>,
}

/// This displays our keyboard
#[function_component(Keyboard)]
fn keyboard(props: &KeyboardProps) -> Html {
    // For each row of keys, we display some HTML
    props.keys.iter().map(|row| {
        // Render the entire row
        let row = row.iter().map(|key| {
            // When a keyboard button is clicked, we propagate it to the main Game through the
            // on_click that is passed in via the launch properties
            let on_click = {
                let on_click = props.on_click.clone();
                let key = key.key;
                Callback::from(move |_| {
                    on_click.emit(key);
                })
            };

            if key.key == '|' {
                html! {
                    <div class="spacer_key"></div>
                }
            } else {
                let (label, class) = match key.key {
                    '\n' => ('↵', "key special_key"),
                    '\x08' => ('⌫', "key special_key"),
                    c => (c, "key"),
                };

                html! {
                    <button class={ class } onclick={ on_click } style={ key.css() }>{ label }</button>
                }
            }

        }).collect::<Html>();

        // Wrap the rows up into a div
        html! {
            <div class="row">{ row }</div>
        }
    }).collect()
}

#[derive(Clone, PartialEq)]
struct Key {
    key: char,
    state: Option<CharGuessResult>,
}

impl Key {
    fn css(&self) -> &'static str {
        self.state.map(|s| s.color_css()).unwrap_or_default()
    }
}

impl From<char> for Key {
    fn from(key: char) -> Self {
        Key { key, state: None }
    }
}

#[derive(Clone, PartialEq, Default)]
struct Guess {
    letters: Vec<GuessLetter>,
}

#[derive(Copy, Clone, PartialEq)]
struct GuessLetter {
    state: Option<CharGuessResult>,
    letter: char,
}

impl Default for GuessLetter {
    fn default() -> Self {
        GuessLetter {
            state: None,
            letter: ' ',
        }
    }
}

impl GuessLetter {
    fn css(&self) -> &'static str {
        self.state.map(|s| s.color_css()).unwrap_or_default()
    }
}

impl From<char> for GuessLetter {
    fn from(letter: char) -> Self {
        GuessLetter {
            state: None,
            letter,
        }
    }
}

#[derive(Eq, PartialEq, Debug, Copy, Clone, Ord, PartialOrd)]
enum CharGuessResult {
    Incorrect = 0,
    WrongPlace = 1,
    Correct = 2,
}

impl CharGuessResult {
    fn color_css(&self) -> &'static str {
        use CharGuessResult::*;

        match self {
            Correct => "background-color: green",
            WrongPlace => "background-color: yellow",
            _ => "background-color: dimgray",
        }
    }
}

impl Display for CharGuessResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use CharGuessResult::*;

        let emoji = match self {
            Correct => "🟩",    // Green square
            WrongPlace => "🟨", // Yellow square
            Incorrect => "⬛",  // Black square
        };

        f.write_str(emoji)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum GameState {
    Won,
    Continue,
    Lost,
}
