//! Helpers for writing Alfred XML output
//!
//! # Example
//!
//! ```
//! extern crate alfred;
//!
//! use std::io;
//!
//! fn write_items() -> io::IoResult<()> {
//!     let mut xmlw = try!(alfred::XMLWriter::new(io::stdout()));
//!
//!     let item1 = alfred::Item::new("Item 1");
//!     let item2 = alfred::ItemBuilder::new("Item 2")
//!                                     .subtitle("Subtitle")
//!                                     .into_item();
//!     let item3 = alfred::ItemBuilder::new("Item 3")
//!                                     .arg("Argument")
//!                                     .subtitle("Subtitle")
//!                                     .icon_filetype("public.folder")
//!                                     .into_item();
//!
//!     try!(xmlw.write_item(&item1));
//!     try!(xmlw.write_item(&item2));
//!     try!(xmlw.write_item(&item3));
//!
//!     let mut stdout = try!(xmlw.close());
//!     stdout.flush()
//! }
//!
//! fn main() {
//!     match write_items() {
//!         Ok(()) => {},
//!         Err(err) => {
//!             let _ = writeln!(io::stderr(), "Error writing items: {}", err);
//!         }
//!     }
//! }
//! ```

// example environment for a script filter:
//
// alfred_preferences = /Users/kevin/Dropbox (Personal)/Alfred.alfredpreferences
// alfred_preferences_localhash = 24e980586e9906f9f08aa9febc3ef05f603e58ef
// alfred_theme = alfred.theme.yosemite
// alfred_theme_background = rgba(255,255,255,0.98)
// alfred_theme_subtext = 0
// alfred_version = 2.5
// alfred_version_build = 299
// alfred_workflow_bundleid = com.tildesoft.alfred.workflow.github-quick-open
// alfred_workflow_cache = /Users/kevin/Library/Caches/com.runningwithcrayons.Alfred-2/Workflow Data/com.tildesoft.alfred.workflow.github-quick-open
// alfred_workflow_data = /Users/kevin/Library/Application Support/Alfred 2/Workflow Data/com.tildesoft.alfred.workflow.github-quick-open
// alfred_workflow_name = GitHub Quick Open
// alfred_workflow_uid = user.workflow.9D443143-3DF7-4596-993E-DA198039EFAB

#![feature(if_let, unsafe_destructor)]
#![warn(missing_doc)]

use std::collections::HashMap;
use std::io;
use std::io::BufferedWriter;
use std::mem;
use std::str;
use std::str::{MaybeOwned, IntoMaybeOwned};

/// Representation of an `<item>`
#[deriving(PartialEq,Eq,Clone)]
pub struct Item<'a> {
    /// Title for the item
    pub title: MaybeOwned<'a>,
    /// Subtitle for the item
    ///
    /// The subtitle may differ based on the active modifier.
    pub subtitle: HashMap<Option<Modifier>,MaybeOwned<'a>>,
    /// Icon for the item
    pub icon: Option<Icon<'a>>,

    /// Identifier for the results
    ///
    /// If given, must be unique among items, and is used for prioritizing
    /// feedback results based on usage. If blank, Alfred presents results in
    /// the order given and does not learn from them.
    pub uid: Option<MaybeOwned<'a>>,
    /// The value that is passed to the next portion of the workflow when this
    /// item is selected
    pub arg: Option<MaybeOwned<'a>>,
    /// What type of result this is
    pub type_: ItemType,

    /// Whether or not the result is valid
    ///
    /// When `false`, actioning the result will populate the search field with
    /// the `autocomplete` value instead.
    pub valid: bool,
    /// Autocomplete data for the item
    ///
    /// This value is populated into the search field if the tab key is
    /// pressed. If `valid = false`, this value is populated if the item is
    /// actioned.
    pub autocomplete: Option<MaybeOwned<'a>>,
    /// What text the user gets when copying the result
    ///
    /// This value is copied if the user presses ⌘C.
    pub text_copy: Option<MaybeOwned<'a>>,
    /// What text the user gets when displaying large type
    ///
    /// This value is displayed if the user presses ⌘L.
    pub text_large_type: Option<MaybeOwned<'a>>,
}

impl<'a> Item<'a> {
    /// Returns a new `Item` with the given title
    pub fn new<S: IntoMaybeOwned<'a>>(title: S) -> Item<'a> {
        Item {
            title: title.into_maybe_owned(),
            subtitle: HashMap::new(),
            icon: None,
            uid: None,
            arg: None,
            type_: DefaultItemType,
            valid: true,
            autocomplete: None,
            text_copy: None,
            text_large_type: None,
        }
    }
}

/// Helper for building `Item` values
#[deriving(Clone)]
pub struct ItemBuilder<'a> {
    item: Item<'a>
}

impl<'a> ItemBuilder<'a> {
    /// Returns a new `ItemBuilder` with the given title
    pub fn new<S: IntoMaybeOwned<'a>>(title: S) -> ItemBuilder<'a> {
        ItemBuilder {
            item: Item::new(title)
        }
    }

    /// Returns the built `Item`
    pub fn into_item(self) -> Item<'a> {
        self.item
    }
}

impl<'a> ItemBuilder<'a> {
    /// Sets the `title` to the given value
    pub fn title<S: IntoMaybeOwned<'a>>(mut self, title: S) -> ItemBuilder<'a> {
        self.item.title = title.into_maybe_owned();
        self
    }

    /// Sets the default `subtitle` to the given value
    ///
    /// This sets the default subtitle, used when no modifier is pressed,
    /// or when no subtitle is provided for the pressed modifier.
    pub fn subtitle<S: IntoMaybeOwned<'a>>(mut self, subtitle: S) -> ItemBuilder<'a> {
        self.item.subtitle.insert(None, subtitle.into_maybe_owned());
        self
    }

    /// Sets the `subtitle` to the given value with the given modifier
    ///
    /// This sets the subtitle to use when the given modifier is pressed.
    pub fn subtitle_mod<S: IntoMaybeOwned<'a>>(mut self, modifier: Modifier, subtitle: S)
                                              -> ItemBuilder<'a> {
        self.item.subtitle.insert(Some(modifier), subtitle.into_maybe_owned());
        self
    }

    /// Sets the `icon` to an image file on disk
    ///
    /// The path is interpreted relative to the workflow directory
    pub fn icon_path<S: IntoMaybeOwned<'a>>(mut self, path: S) -> ItemBuilder<'a> {
        self.item.icon = Some(PathIcon(path.into_maybe_owned()));
        self
    }

    /// Sets the `icon` to the icon for a given file on disk
    ///
    /// The path is interpreted relative to the workflow directory
    pub fn icon_file<S: IntoMaybeOwned<'a>>(mut self, path: S) -> ItemBuilder<'a> {
        self.item.icon = Some(FileIcon(path.into_maybe_owned()));
        self
    }

    /// Sets the `icon` to the icon for a given file type
    ///
    /// The type is a UTI, such as "public.jpeg".
    pub fn icon_filetype<S: IntoMaybeOwned<'a>>(mut self, filetype: S) -> ItemBuilder<'a> {
        self.item.icon = Some(FileTypeIcon(filetype.into_maybe_owned()));
        self
    }

    /// Sets the `uid` to the given value
    pub fn uid<S: IntoMaybeOwned<'a>>(mut self, uid: S) -> ItemBuilder<'a> {
        self.item.uid = Some(uid.into_maybe_owned());
        self
    }

    /// Sets the `arg` to the given value
    pub fn arg<S: IntoMaybeOwned<'a>>(mut self, arg: S) -> ItemBuilder<'a> {
        self.item.arg = Some(arg.into_maybe_owned());
        self
    }

    /// Sets the `type` to the given value
    pub fn type_(mut self, type_: ItemType) -> ItemBuilder<'a> {
        self.item.type_ = type_;
        self
    }

    /// Sets `valid` to `true`
    pub fn valid(mut self) -> ItemBuilder<'a> {
        self.item.valid = true;
        self
    }

    /// Sets `valid` to `false`
    pub fn invalid(mut self) -> ItemBuilder<'a> {
        self.item.valid = false;
        self
    }

    /// Sets `valid` to the given value
    pub fn set_valid(mut self, valid: bool) -> ItemBuilder<'a> {
        self.item.valid = valid;
        self
    }

    /// Sets `autocomplete` to the given value
    pub fn autocomplete<S: IntoMaybeOwned<'a>>(mut self, autocomplete: S) -> ItemBuilder<'a> {
        self.item.autocomplete = Some(autocomplete.into_maybe_owned());
        self
    }

    /// Sets `text_copy` to the given value
    pub fn set_text_copy<S: IntoMaybeOwned<'a>>(mut self, text: S) -> ItemBuilder<'a> {
        self.item.text_copy = Some(text.into_maybe_owned());
        self
    }

    /// Sets `text_large_type` to the given value
    pub fn set_text_large_type<S: IntoMaybeOwned<'a>>(mut self, text: S) -> ItemBuilder<'a> {
        self.item.text_large_type = Some(text.into_maybe_owned());
        self
    }
}

impl<'a> ItemBuilder<'a> {
    /// Unsets the default `subtitle`
    ///
    /// This unsets the default subtitle.
    pub fn unset_subtitle(mut self) -> ItemBuilder<'a> {
        self.item.subtitle.remove(&None);
        self
    }

    /// Unsets the `subtitle` for the given modifier
    ///
    /// This unsets the subtitle that's used when the given modifier is pressed.
    pub fn unset_subtitle_mod(mut self, modifier: Modifier) -> ItemBuilder<'a> {
        self.item.subtitle.remove(&Some(modifier));
        self
    }

    /// Clears the `subtitle` for all modifiers
    ///
    /// This unsets both the default subtitle and the per-modifier subtitles.
    pub fn clear_subtitle(mut self) -> ItemBuilder<'a> {
        self.item.subtitle.clear();
        self
    }

    /// Unsets the `icon`
    pub fn unset_icon(mut self) -> ItemBuilder<'a> {
        self.item.icon = None;
        self
    }

    /// Unsets the `uid`
    pub fn unset_uid(mut self) -> ItemBuilder<'a> {
        self.item.uid = None;
        self
    }

    /// Unsets the `arg`
    pub fn unset_arg(mut self) -> ItemBuilder<'a> {
        self.item.arg = None;
        self
    }

    // `type` doesn't need unsetting, it uses a default of DefaultItemType instead

    /// Unsets `autocomplete`
    pub fn unset_autocomplete(mut self) -> ItemBuilder<'a> {
        self.item.autocomplete = None;
        self
    }

    /// Unsets `text_copy`
    pub fn unset_text_copy(mut self) -> ItemBuilder<'a> {
        self.item.text_copy = None;
        self
    }

    /// Unsets `text_large_type`
    pub fn unset_text_large_type(mut self) -> ItemBuilder<'a> {
        self.item.text_large_type = None;
        self
    }
}

/// Keyboard modifiers
// As far as I can tell, Alfred doesn't support modifier combinations.
#[deriving(Clone,Show,Hash,PartialEq,Eq)]
pub enum Modifier {
    /// Command key
    CmdModifier,
    /// Option/Alt key
    AltModifier,
    /// Control key
    CtrlModifier,
    /// Shift key
    ShiftModifier,
    /// Fn key
    FnModifier
}

/// Item icons
#[deriving(PartialEq,Eq,Clone)]
pub enum Icon<'a> {
    /// Path to an image file on disk relative to the workflow directory
    PathIcon(str::MaybeOwned<'a>),
    /// Path to a file whose icon will be used
    FileIcon(str::MaybeOwned<'a>),
    /// UTI for a file type to use (e.g. public.folder)
    FileTypeIcon(str::MaybeOwned<'a>)
}

/// Item types
#[deriving(PartialEq,Eq,Clone)]
pub enum ItemType {
    /// Default type for an item
    DefaultItemType,
    /// Type representing a file
    ///
    /// Alredy checks that the file exists on disk, and hides the result if it
    /// does not.
    FileItemType,
    /// Type representing a file, with filesystem checks skipped
    ///
    /// Similar to `FileItemType` but skips the check to ensure the file
    /// exists.
    FileSkipCheckItemType
}

/// Helper struct used to manage the XML serialization of `Item`s
///
/// When the `XMLWriter` is first created, the XML header is immediately
/// written. When the `XMLWriter` is dropped, the XML footer is written
/// and the `Writer` is flushed.
///
/// Any errors produced by writing the footer are silently ignored. The
/// `close()` method can be used to return any such error.
pub struct XMLWriter<'a, W: Writer + 'a> {
    // Option so close() can remove it
    // Otherwise this must alwyas be Some()
    w: Option<W>,
    last_err: Option<io::IoError>
}

impl<'a, W: Writer + 'a> XMLWriter<'a, W> {
    /// Returns a new `XMLWriter` that writes to the given `Writer`
    ///
    /// The XML header is written immediately.
    pub fn new(mut w: W) -> io::IoResult<XMLWriter<'a, W>> {
        match w.write_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<items>\n") {
            Ok(()) => {
                Ok(XMLWriter {
                    w: Some(w),
                    last_err: None
                })
            }
            Err(err) => Err(err)
        }
    }

    /// Writes an `Item` to the underlying `Writer`
    ///
    /// If a previous write produced an error, any subsequent write will do
    /// nothing and return the same error. This is because the previous write
    /// may have partially completed, and attempting to write any more data
    /// will be unlikely to work properly.
    pub fn write_item(&mut self, item: &Item) -> io::IoResult<()> {
        if let Some(ref err) = self.last_err {
            return Err(err.clone());
        }
        let result = item.write_xml(self.w.as_mut().unwrap(), 1);
        if let Err(ref err) = result {
            self.last_err = Some(err.clone());
        }
        result
    }

    /// Consumes the `XMLWriter` and writes the XML footer
    ///
    /// This method can be used to get any error resulting from writing the
    /// footer. If this method is not used, the footer will be written when the
    /// `XMLWriter` is dropped and any error will be silently ignored.
    ///
    /// As with `write_item()`, if a previous invocation of `write_item()`
    /// returned an error, `close()` will return the same error without
    /// attempting to write the XML footer.
    ///
    /// When this method is used, the XML footer is written, but the `Writer`
    /// is not flushed. When the `XMLWriter` is dropped without calling
    /// `close()`, the `Writer` is flushed after the footer is written.
    pub fn close(mut self) -> io::IoResult<W> {
        let last_err = self.last_err.take();
        let mut w = self.w.take().unwrap();
        unsafe { mem::forget(self); }
        if let Some(err) = last_err {
            return Err(err);
        }
        try!(write_footer(&mut w));
        Ok(w)
    }
}

fn write_footer<'a, W: Writer + 'a>(w: &'a mut W) -> io::IoResult<()> {
    w.write_str("</items>\n")
}

#[unsafe_destructor]
impl<'a, W: Writer + 'a> Drop for XMLWriter<'a, W> {
    fn drop(&mut self) {
        if self.last_err.is_some() {
            return;
        }
        let mut w = self.w.take().unwrap();
        if write_footer(&mut w).is_ok() {
            let _ = w.flush();
        }
    }
}

/// Writes a complete XML document representing the `Item`s to the `Writer`
///
/// The `Writer` is flushed after the XML document is written.
pub fn write_items<W: Writer>(w: W, items: &[Item]) -> io::IoResult<()> {
    let mut xmlw = try!(XMLWriter::new(w));
    for item in items.iter() {
        try!(xmlw.write_item(item));
    }
    let mut w = try!(xmlw.close());
    w.flush()
}

impl<'a> Item<'a> {
    /// Writes the XML fragment representing the `Item` to the `Writer`
    ///
    /// `XMLWriter` should be used instead if at all possible, in order to
    /// write the XML header/footer and maintain proper error discipline.
    pub fn write_xml(&self, w: &mut io::Writer, indent: uint) -> io::IoResult<()> {
        fn write_indent(w: &mut io::Writer, indent: uint) -> io::IoResult<()> {
            for _ in range(0, indent) {
                try!(w.write_str("    "));
            }
            Ok(())
        }

        let mut w = BufferedWriter::with_capacity(512, w);

        try!(write_indent(&mut w, indent));
        try!(w.write_str("<item"));
        if let Some(ref uid) = self.uid {
            try!(write!(&mut w, r#" uid="{}""#, encode_entities(uid.as_slice())));
        }
        if let Some(ref arg) = self.arg {
            try!(write!(&mut w, r#" arg="{}""#, encode_entities(arg.as_slice())));
        }
        match self.type_ {
            DefaultItemType => {}
            FileItemType => {
                try!(w.write_str(r#" type="file""#));
            }
            FileSkipCheckItemType => {
                try!(w.write_str(r#" type="file:skipcheck""#));
            }
        }
        if !self.valid {
            try!(w.write_str(r#" valid="no""#));
        }
        if let Some(ref auto) = self.autocomplete {
            try!(write!(&mut w, r#" autocomplete="{}""#, encode_entities(auto.as_slice())));
        }
        try!(w.write_str(">\n"));

        try!(write_indent(&mut w, indent+1));
        try!(write!(&mut w, "<title>{}</title>\n", encode_entities(self.title.as_slice())));

        for (modifier, subtitle) in self.subtitle.iter() {
            try!(write_indent(&mut w, indent+1));
            if let Some(modifier) = *modifier {
                try!(write!(w, r#"<subtitle mod="{}">"#, match modifier {
                    CmdModifier => "cmd",
                    AltModifier => "alt",
                    CtrlModifier => "ctrl",
                    ShiftModifier => "shift",
                    FnModifier => "fn"
                }));
            } else {
                try!(w.write_str("<subtitle>"));
            }
            try!(write!(w, "{}</subtitle>\n", encode_entities(subtitle.as_slice())));
        }

        if let Some(ref icon) = self.icon {
            try!(write_indent(&mut w, indent+1));
            match *icon {
                PathIcon(ref s) => {
                    try!(write!(&mut w, "<icon>{}</icon>\n", encode_entities(s.as_slice())));
                }
                FileIcon(ref s) => {
                    try!(write!(&mut w, "<icon type=\"fileicon\">{}</icon>\n",
                                    encode_entities(s.as_slice())));
                }
                FileTypeIcon(ref s) => {
                    try!(write!(&mut w, "<icon type=\"filetype\">{}</icon>\n",
                                    encode_entities(s.as_slice())));
                }
            }
        }

        if let Some(ref text) = self.text_copy {
            try!(write_indent(&mut w, indent+1));
            try!(write!(w, "<text type=\"copy\">{}</text>\n", encode_entities(text.as_slice())));
        }
        if let Some(ref text) = self.text_large_type {
            try!(write_indent(&mut w, indent+1));
            try!(write!(w, "<text type=\"largetype\">{}</text>\n",
                        encode_entities(text.as_slice())));
        }

        try!(write_indent(&mut w, indent));
        try!(w.write_str("</item>\n"));

        w.flush()
    }
}

fn encode_entities<'a>(s: &'a str) -> str::MaybeOwned<'a> {
    fn encode_entity(c: char) -> Option<&'static str> {
        Some(match c {
            '<' => "&lt;",
            '>' => "&gt;",
            '"' => "&quot;",
            '&' => "&amp;",
            '\0'...'\x08' |
            '\x0B'...'\x0C' |
            '\x0E'...'\x1F' |
            '\uFFFE' | '\uFFFF' => {
                // these are all invalid characters in XML
                "\uFFFD"
            }
            _ => return None
        })
    }

    if s.chars().any(|c| encode_entity(c).is_some()) {
        let mut res = String::with_capacity(s.len());
        for c in s.chars() {
            match encode_entity(c) {
                Some(ent) => res.push_str(ent),
                None => res.push(c)
            }
        }
        str::Owned(res)
    } else {
        str::Slice(s)
    }
}
