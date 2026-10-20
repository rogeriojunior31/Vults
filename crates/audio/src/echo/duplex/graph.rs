use super::*;
use libpulse_sys as ffi;
use std::{
    cell::RefCell,
    ffi::{c_char, c_void},
    rc::Rc,
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Node {
    pub index: u32,
    pub serial: u64,
    pub name: String,
    pub group: Option<String>,
    pub module: Option<u32>,
    pub port: Option<String>,
    pub rate: u32,
    pub channels: u8,
    pub monitor: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Edge {
    pub index: u32,
    pub serial: u64,
    pub module: Option<u32>,
    pub device: u32,
}
#[derive(Clone, Debug, Default)]
pub(super) struct Graph {
    pub cookie: Option<u32>,
    pub pipewire: bool,
    pub sources: Vec<Node>,
    pub sinks: Vec<Node>,
    pub capture: Vec<Edge>,
    pub playback: Vec<Edge>,
    pub modules: Vec<u32>,
    done: u8,
    failed: bool,
}
pub(super) struct Request {
    pub graph: Rc<RefCell<Graph>>,
    _ops: Vec<raw::Op>,
}
impl Graph {
    pub fn completed(&self) -> Result<bool, Failure> {
        if self.failed {
            Err(Failure::Route)
        } else {
            Ok(self.done == 6)
        }
    }
}
fn string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    // Native info strings are NUL-terminated; bound every metadata copy before allocating.
    let len = unsafe { libc::strnlen(ptr, 1025) };
    if len == 0 || len > 1024 {
        return None;
    }
    std::str::from_utf8(unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), len) })
        .ok()
        .map(str::to_owned)
}
fn prop(ptr: *mut ffi::pa_proplist, key: &std::ffi::CStr) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        string(unsafe { ffi::pa_proplist_gets(ptr, key.as_ptr()) })
    }
}
fn serial(ptr: *mut ffi::pa_proplist) -> u64 {
    prop(ptr, c"object.serial")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}
fn module(index: u32) -> Option<u32> {
    (index != u32::MAX).then_some(index)
}
fn source(s: &ffi::pa_source_info) -> Node {
    Node {
        index: s.index,
        serial: serial(s.proplist),
        name: string(s.name).unwrap_or_default(),
        group: prop(s.proplist, c"node.group"),
        module: module(s.owner_module),
        port: unsafe { s.active_port.as_ref() }.and_then(|p| string(p.name)),
        rate: s.sample_spec.rate,
        channels: s.sample_spec.channels,
        monitor: s.monitor_of_sink != u32::MAX,
    }
}
fn sink(s: &ffi::pa_sink_info) -> Node {
    Node {
        index: s.index,
        serial: serial(s.proplist),
        name: string(s.name).unwrap_or_default(),
        group: prop(s.proplist, c"node.group"),
        module: module(s.owner_module),
        port: unsafe { s.active_port.as_ref() }.and_then(|p| string(p.name)),
        rate: s.sample_spec.rate,
        channels: s.sample_spec.channels,
        monitor: false,
    }
}
fn list<T>(data: *mut c_void, item: *const T, eol: i32, push: impl FnOnce(&mut Graph, &T)) {
    // Each operation retains this graph; cancellation precedes releasing it on the same worker.
    let graph = unsafe { &*data.cast::<RefCell<Graph>>() };
    let mut g = graph.borrow_mut();
    if eol != 0 {
        g.done += 1;
        g.failed |= eol < 0;
        return;
    }
    if g.sources.len() + g.sinks.len() + g.capture.len() + g.playback.len() + g.modules.len() >= 4096 {
        g.failed = true;
        return;
    }
    if let Some(item) = unsafe { item.as_ref() } {
        push(&mut g, item);
    } else {
        g.failed = true;
    }
}
extern "C" fn sources(
    _: *mut ffi::pa_context,
    item: *const ffi::pa_source_info,
    eol: i32,
    data: *mut c_void,
) {
    list(data, item, eol, |g, s| g.sources.push(source(s)));
}
extern "C" fn sinks(_: *mut ffi::pa_context, item: *const ffi::pa_sink_info, eol: i32, data: *mut c_void) {
    list(data, item, eol, |g, s| g.sinks.push(sink(s)));
}
extern "C" fn capture(
    _: *mut ffi::pa_context,
    item: *const ffi::pa_source_output_info,
    eol: i32,
    data: *mut c_void,
) {
    list(data, item, eol, |g, s| {
        g.capture.push(Edge {
            index: s.index,
            serial: serial(s.proplist),
            module: module(s.owner_module),
            device: s.source,
        })
    });
}
extern "C" fn playback(
    _: *mut ffi::pa_context,
    item: *const ffi::pa_sink_input_info,
    eol: i32,
    data: *mut c_void,
) {
    list(data, item, eol, |g, s| {
        g.playback.push(Edge {
            index: s.index,
            serial: serial(s.proplist),
            module: module(s.owner_module),
            device: s.sink,
        })
    });
}
extern "C" fn modules(
    _: *mut ffi::pa_context,
    item: *const ffi::pa_module_info,
    eol: i32,
    data: *mut c_void,
) {
    list(data, item, eol, |g, s| {
        let m = pulseaudio::protocol::ModuleInfo {
            index: s.index,
            name: std::ffi::CString::new(string(s.name).unwrap_or_default()).unwrap_or_default(),
            argument: string(s.argument).and_then(|a| std::ffi::CString::new(a).ok()),
            ..Default::default()
        };
        if super::super::webrtc(&m) {
            g.modules.push(s.index);
        }
    });
}
extern "C" fn server(_: *mut ffi::pa_context, item: *const ffi::pa_server_info, data: *mut c_void) {
    let graph = unsafe { &*data.cast::<RefCell<Graph>>() };
    let mut g = graph.borrow_mut();
    if let Some(s) = unsafe { item.as_ref() } {
        g.cookie = Some(s.cookie);
        g.pipewire = string(s.server_name).is_some_and(|n| n.starts_with("PulseAudio (on PipeWire "));
    } else {
        g.failed = true;
    }
    g.done += 1;
}
fn data(graph: &Rc<RefCell<Graph>>) -> (Rc<RefCell<Graph>>, *mut c_void) {
    (graph.clone(), Rc::as_ptr(graph).cast_mut().cast())
}
pub(super) fn request(context: &raw::Context) -> Result<Request, Failure> {
    let graph = Rc::new(RefCell::new(Graph::default()));
    let mut ops = Vec::new();
    let ctx = context.ptr.as_ptr();
    let (owner, ptr) = data(&graph);
    ops.push(raw::Op::new(
        unsafe { ffi::pa_context_get_server_info(ctx, Some(server), ptr) },
        owner,
    )?);
    let (owner, ptr) = data(&graph);
    ops.push(raw::Op::new(
        unsafe { ffi::pa_context_get_source_info_list(ctx, Some(sources), ptr) },
        owner,
    )?);
    let (owner, ptr) = data(&graph);
    ops.push(raw::Op::new(
        unsafe { ffi::pa_context_get_sink_info_list(ctx, Some(sinks), ptr) },
        owner,
    )?);
    let (owner, ptr) = data(&graph);
    ops.push(raw::Op::new(
        unsafe { ffi::pa_context_get_source_output_info_list(ctx, Some(capture), ptr) },
        owner,
    )?);
    let (owner, ptr) = data(&graph);
    ops.push(raw::Op::new(
        unsafe { ffi::pa_context_get_sink_input_info_list(ctx, Some(playback), ptr) },
        owner,
    )?);
    let (owner, ptr) = data(&graph);
    ops.push(raw::Op::new(
        unsafe { ffi::pa_context_get_module_info_list(ctx, Some(modules), ptr) },
        owner,
    )?);
    Ok(Request { graph, _ops: ops })
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Pinned {
    pub source: Node,
    pub sink: Node,
    pub capture: Option<(Edge, Node)>,
    pub playback: Option<(Edge, Node)>,
    pub cookie: u32,
}
fn one<T>(mut items: impl Iterator<Item = T>) -> Option<T> {
    let item = items.next()?;
    items.next().is_none().then_some(item)
}
impl Pinned {
    pub fn from_graph(route: &Route, g: &Graph) -> Result<Self, Failure> {
        if !g.pipewire || g.cookie != Some(route.server) {
            return Err(Failure::Route);
        }
        let select = |nodes: &[Node], expected: &super::super::Endpoint| {
            one(nodes.iter().filter(|n| {
                n.index == expected.index
                    && n.serial == expected.serial
                    && n.name == expected.name
                    && n.serial > 0
            }))
            .cloned()
        };
        let source = select(&g.sources, &route.source).ok_or(Failure::Route)?;
        let sink = select(&g.sinks, &route.sink).ok_or(Failure::Route)?;
        if source.monitor
            || ![&source, &sink]
                .iter()
                .all(|n| (1..=2).contains(&n.channels) && (16000..=96000).contains(&n.rate))
        {
            return Err(Failure::Route);
        }
        let (capture, playback) = if let Some(pair) = &route.echo {
            if !g.modules.contains(&pair.module)
                || source.module != Some(pair.module)
                || sink.module != Some(pair.module)
                || source.group.as_deref() != Some(&pair.group)
                || sink.group != source.group
            {
                return Err(Failure::Route);
            }
            let edge = |edges: &[Edge], nodes: &[Node]| {
                let e = one(edges
                    .iter()
                    .filter(|e| e.module == Some(pair.module) && e.serial > 0))?;
                let n = one(nodes.iter().filter(|n| n.index == e.device && n.serial > 0))?;
                Some((e.clone(), n.clone()))
            };
            (
                Some(edge(&g.capture, &g.sources).ok_or(Failure::Route)?),
                Some(edge(&g.playback, &g.sinks).ok_or(Failure::Route)?),
            )
        } else {
            (None, None)
        };
        Ok(Self {
            source,
            sink,
            capture,
            playback,
            cookie: route.server,
        })
    }
}
