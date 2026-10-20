//! Worker-local native handles. Callback storage is borrowed by C and reclaimed only after cancellation.
use super::*;
pub(super) use ffi::{
    pa_buffer_attr as BufferAttr, pa_context_state_t as ContextState, pa_sample_format_t as Format,
    pa_sample_spec as Spec, pa_stream_state_t as StreamState,
};
use libpulse_sys as ffi;
use std::{
    any::Any,
    cell::Cell,
    ffi::{CString, c_void},
    ptr::{self, NonNull},
    rc::Rc,
};
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct TimingInfo {
    pub read_index: i64,
    pub write_index: i64,
    pub read_index_corrupt: i32,
    pub write_index_corrupt: i32,
    pub sink_usec: u64,
}
pub(super) struct Op {
    ptr: NonNull<ffi::pa_operation>,
    _data: Box<dyn Any>,
}
impl Op {
    pub fn new<T: 'static>(ptr: *mut ffi::pa_operation, data: T) -> Result<Self, Failure> {
        Ok(Self {
            ptr: NonNull::new(ptr).ok_or(Failure::Device)?,
            _data: Box::new(data),
        })
    }
    pub fn done(&self) -> bool {
        unsafe { ffi::pa_operation_get_state(self.ptr.as_ptr()) != ffi::pa_operation_state_t::Running }
    }
}
impl Drop for Op {
    fn drop(&mut self) {
        unsafe {
            ffi::pa_operation_cancel(self.ptr.as_ptr());
            ffi::pa_operation_unref(self.ptr.as_ptr());
        }
    }
}
pub(super) struct Ack {
    pub _op: Op,
    pub result: Rc<Cell<Option<bool>>>,
}
fn ack_data() -> (Rc<Cell<Option<bool>>>, *mut c_void) {
    let data = Rc::new(Cell::new(None));
    let ptr = Rc::as_ptr(&data).cast_mut().cast();
    (data, ptr)
}
extern "C" fn stream_ack(_: *mut ffi::pa_stream, ok: i32, data: *mut c_void) {
    unsafe {
        (&*data.cast::<Cell<Option<bool>>>()).set(Some(ok != 0));
    }
}
extern "C" fn context_ack(_: *mut ffi::pa_context, ok: i32, data: *mut c_void) {
    unsafe {
        (&*data.cast::<Cell<Option<bool>>>()).set(Some(ok != 0));
    }
}
pub(super) struct Mainloop(NonNull<ffi::pa_mainloop>);
impl Mainloop {
    pub fn new() -> Result<Self, Failure> {
        NonNull::new(unsafe { ffi::pa_mainloop_new() })
            .map(Self)
            .ok_or(Failure::Device)
    }
    pub fn iterate(&mut self) -> Result<bool, Failure> {
        let mut code = 0;
        let n = unsafe { ffi::pa_mainloop_iterate(self.0.as_ptr(), 0, &mut code) };
        if n < 0 { Err(Failure::Device) } else { Ok(n > 0) }
    }
}
impl Drop for Mainloop {
    fn drop(&mut self) {
        unsafe {
            ffi::pa_mainloop_free(self.0.as_ptr());
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Facility {
    Sink,
    Source,
    SinkInput,
    SourceOutput,
    Module,
    Server,
    Card,
    Other,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Change {
    New,
    Changed,
    Removed,
}
type Subscription = Box<dyn FnMut(Option<Facility>, Option<Change>, u32)>;
extern "C" fn subscribed(_: *mut ffi::pa_context, event: u32, index: u32, data: *mut c_void) {
    let facility = match event & 15 {
        0 => Facility::Sink,
        1 => Facility::Source,
        2 => Facility::SinkInput,
        3 => Facility::SourceOutput,
        4 => Facility::Module,
        7 => Facility::Server,
        9 => Facility::Card,
        _ => Facility::Other,
    };
    let operation = match event & 0x30 {
        0 => Change::New,
        0x10 => Change::Changed,
        _ => Change::Removed,
    };
    // This box stays on the owning worker until the callback is detached from its context.
    unsafe {
        (&mut *data.cast::<Subscription>())(Some(facility), Some(operation), index);
    }
}
pub(super) struct Context {
    pub ptr: NonNull<ffi::pa_context>,
    callback: Option<Box<Subscription>>,
    closed: bool,
}
impl Context {
    pub fn new(mainloop: &Mainloop) -> Result<Self, Failure> {
        NonNull::new(unsafe {
            ffi::pa_context_new(
                ffi::pa_mainloop_get_api(mainloop.0.as_ptr()),
                c"Vults duplex".as_ptr(),
            )
        })
        .map(|ptr| Self {
            ptr,
            callback: None,
            closed: false,
        })
        .ok_or(Failure::Device)
    }
    pub fn connect(&mut self, endpoint: &str) -> Result<(), Failure> {
        let name = CString::new(endpoint).map_err(|_| Failure::Input)?;
        ok(unsafe {
            ffi::pa_context_connect(
                self.ptr.as_ptr(),
                name.as_ptr(),
                ffi::PA_CONTEXT_NOAUTOSPAWN,
                ptr::null(),
            )
        })
    }
    pub fn state(&self) -> ContextState {
        unsafe { ffi::pa_context_get_state(self.ptr.as_ptr()) }
    }
    pub fn set_subscribe_callback(&mut self, callback: Option<Subscription>) {
        unsafe {
            ffi::pa_context_set_subscribe_callback(self.ptr.as_ptr(), None, ptr::null_mut());
        }
        self.callback = callback.map(Box::new);
        if let Some(callback) = &mut self.callback {
            unsafe {
                ffi::pa_context_set_subscribe_callback(
                    self.ptr.as_ptr(),
                    Some(subscribed),
                    (&mut **callback as *mut Subscription).cast(),
                );
            }
        }
    }
    pub fn subscribe(&mut self) -> Result<Ack, Failure> {
        let (data, ptr) = ack_data();
        let result = data.clone();
        let op = unsafe {
            ffi::pa_context_subscribe(
                self.ptr.as_ptr(),
                ffi::PA_SUBSCRIPTION_MASK_ALL,
                Some(context_ack),
                ptr,
            )
        };
        Ok(Ack {
            result,
            _op: Op::new(op, data)?,
        })
    }
    pub fn disconnect(&mut self) {
        if !self.closed {
            unsafe {
                ffi::pa_context_disconnect(self.ptr.as_ptr());
            }
            self.closed = true;
        }
    }
}
impl Drop for Context {
    fn drop(&mut self) {
        self.set_subscribe_callback(None);
        self.disconnect();
        unsafe {
            ffi::pa_context_unref(self.ptr.as_ptr());
        }
    }
}
type Notification = Box<dyn FnMut()>;
extern "C" fn notified(_: *mut ffi::pa_stream, data: *mut c_void) {
    unsafe {
        (&mut *data.cast::<Notification>())();
    }
}
pub(super) struct Stream {
    ptr: NonNull<ffi::pa_stream>,
    moved: Option<Box<Notification>>,
    overflow: Option<Box<Notification>>,
    underflow: Option<Box<Notification>>,
    pub timing_op: Option<Op>,
}
impl Stream {
    pub fn new(context: &mut Context, name: &str) -> Result<Self, Failure> {
        let name = CString::new(name).map_err(|_| Failure::Input)?;
        let spec = Spec {
            format: Format::S16le,
            rate: RATE,
            channels: 1,
        };
        let ptr = NonNull::new(unsafe {
            ffi::pa_stream_new(context.ptr.as_ptr(), name.as_ptr(), &spec, ptr::null())
        })
        .ok_or(Failure::Device)?;
        Ok(Self {
            ptr,
            moved: None,
            overflow: None,
            underflow: None,
            timing_op: None,
        })
    }
    pub fn attach(&mut self, device: &str, record: bool) -> Result<(), Failure> {
        let name = CString::new(device).map_err(|_| Failure::Input)?;
        let attr = BufferAttr {
            maxlength: RATE / 5 * 2,
            tlength: RATE / 10 * 2,
            prebuf: 0,
            minreq: RATE / 100 * 2,
            fragsize: RATE / 50 * 2,
        };
        let flags =
            ffi::PA_STREAM_START_CORKED | ffi::PA_STREAM_DONT_MOVE | ffi::PA_STREAM_AUTO_TIMING_UPDATE;
        ok(unsafe {
            if record {
                ffi::pa_stream_connect_record(self.ptr.as_ptr(), name.as_ptr(), &attr, flags)
            } else {
                ffi::pa_stream_connect_playback(
                    self.ptr.as_ptr(),
                    name.as_ptr(),
                    &attr,
                    flags,
                    ptr::null(),
                    ptr::null_mut(),
                )
            }
        })
    }
    pub fn state(&self) -> StreamState {
        unsafe { ffi::pa_stream_get_state(self.ptr.as_ptr()) }
    }
    pub fn get_device_index(&self) -> Option<u32> {
        let index = unsafe { ffi::pa_stream_get_device_index(self.ptr.as_ptr()) };
        (index != u32::MAX).then_some(index)
    }
    pub fn spec_matches(&self) -> bool {
        unsafe {
            ffi::pa_stream_get_sample_spec(self.ptr.as_ptr())
                .as_ref()
                .is_some_and(|s| s.format == Format::S16le && s.channels == 1 && s.rate == RATE)
        }
    }
    pub fn suspended(&self) -> bool {
        unsafe { ffi::pa_stream_is_suspended(self.ptr.as_ptr()) != 0 }
    }
    pub fn uncork(&mut self) -> Result<Ack, Failure> {
        self.ack(false)
    }
    pub fn drain(&mut self) -> Result<Ack, Failure> {
        self.ack(true)
    }
    fn ack(&mut self, drain: bool) -> Result<Ack, Failure> {
        let (data, ptr) = ack_data();
        let result = data.clone();
        let operation = unsafe {
            if drain {
                ffi::pa_stream_drain(self.ptr.as_ptr(), Some(stream_ack), ptr)
            } else {
                ffi::pa_stream_cork(self.ptr.as_ptr(), 0, Some(stream_ack), ptr)
            }
        };
        Ok(Ack {
            result,
            _op: Op::new(operation, data)?,
        })
    }
    pub fn update_timing(&mut self) -> Result<(), Failure> {
        if self.timing_op.as_ref().is_none_or(Op::done) {
            let op = unsafe { ffi::pa_stream_update_timing_info(self.ptr.as_ptr(), None, ptr::null_mut()) };
            self.timing_op = Some(Op::new(op, ())?);
        }
        Ok(())
    }
    pub fn timing(&self) -> Option<TimingInfo> {
        unsafe {
            ffi::pa_stream_get_timing_info(self.ptr.as_ptr())
                .as_ref()
                .map(|i| TimingInfo {
                    read_index: i.read_index,
                    write_index: i.write_index,
                    read_index_corrupt: i.read_index_corrupt,
                    write_index_corrupt: i.write_index_corrupt,
                    sink_usec: i.sink_usec,
                })
        }
    }
    pub fn writable_size(&self) -> Option<usize> {
        let size = unsafe { ffi::pa_stream_writable_size(self.ptr.as_ptr()) };
        (size != usize::MAX).then_some(size)
    }
    pub fn write_copy(&mut self, bytes: &[u8]) -> Result<(), Failure> {
        ok(unsafe {
            ffi::pa_stream_write(
                self.ptr.as_ptr(),
                bytes.as_ptr().cast(),
                bytes.len(),
                None,
                0,
                ffi::PA_SEEK_RELATIVE,
            )
        })
    }
    pub fn read(&mut self) -> Result<Option<Vec<u8>>, Failure> {
        let mut data = ptr::null();
        let mut size = 0;
        ok(unsafe { ffi::pa_stream_peek(self.ptr.as_ptr(), &mut data, &mut size) })?;
        if size == 0 {
            return Ok(None);
        }
        if data.is_null() || size > RATE as usize / 5 * 2 || !size.is_multiple_of(2) {
            return Err(Failure::Overflow);
        }
        // The data is borrowed only until discard; the worker never pumps callbacks in between.
        let bytes = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size) }.to_vec();
        ok(unsafe { ffi::pa_stream_drop(self.ptr.as_ptr()) })?;
        Ok(Some(bytes))
    }
    pub fn disconnect(&mut self) -> Result<(), Failure> {
        self.timing_op.take();
        ok(unsafe { ffi::pa_stream_disconnect(self.ptr.as_ptr()) })
    }
    pub fn clear_callbacks(&mut self) {
        unsafe {
            ffi::pa_stream_set_moved_callback(self.ptr.as_ptr(), None, ptr::null_mut());
            ffi::pa_stream_set_overflow_callback(self.ptr.as_ptr(), None, ptr::null_mut());
            ffi::pa_stream_set_underflow_callback(self.ptr.as_ptr(), None, ptr::null_mut());
        }
        self.moved = None;
        self.overflow = None;
        self.underflow = None;
    }
    pub fn on_moved(&mut self, callback: Notification) {
        self.moved = Some(Box::new(callback));
        if let Some(callback) = &mut self.moved {
            unsafe {
                ffi::pa_stream_set_moved_callback(
                    self.ptr.as_ptr(),
                    Some(notified),
                    (&mut **callback as *mut Notification).cast(),
                );
            }
        }
    }
    pub fn on_overflow(&mut self, callback: Notification) {
        self.overflow = Some(Box::new(callback));
        if let Some(callback) = &mut self.overflow {
            unsafe {
                ffi::pa_stream_set_overflow_callback(
                    self.ptr.as_ptr(),
                    Some(notified),
                    (&mut **callback as *mut Notification).cast(),
                );
            }
        }
    }
    pub fn on_underflow(&mut self, callback: Notification) {
        self.underflow = Some(Box::new(callback));
        if let Some(callback) = &mut self.underflow {
            unsafe {
                ffi::pa_stream_set_underflow_callback(
                    self.ptr.as_ptr(),
                    Some(notified),
                    (&mut **callback as *mut Notification).cast(),
                );
            }
        }
    }
}
impl Drop for Stream {
    fn drop(&mut self) {
        self.timing_op.take();
        self.clear_callbacks();
        unsafe {
            ffi::pa_stream_unref(self.ptr.as_ptr());
        }
    }
}
fn ok(code: i32) -> Result<(), Failure> {
    if code < 0 { Err(Failure::Device) } else { Ok(()) }
}
