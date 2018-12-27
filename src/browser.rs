use crate::ffi;
use crate::DNSServiceError;
use std::ffi::{CString, CStr, c_void};
use std::os::raw::c_char;
use std::mem;
use std::ptr;

pub struct DNSServiceBrowserBuilder {
    regtype: String,
    domain: Option<String>,
}

impl DNSServiceBrowserBuilder {
    pub fn new(regtype: &str) -> DNSServiceBrowserBuilder {
        DNSServiceBrowserBuilder {
            regtype: String::from(regtype),
            domain: None,
        }
    }

    pub fn with_domain(mut self, domain: &str) -> DNSServiceBrowserBuilder {
        self.domain = Some(String::from(domain));
        self
    }

    pub fn build(self) -> Result<DNSServiceBrowser, DNSServiceError> {
        unsafe {
            let service = DNSServiceBrowser {
                regtype: self.regtype,
                domain: self.domain,
                raw: mem::zeroed(),
                // TODO: replace this? think it might live forever
                reply_callback: Box::new(|_, _, _, _, _| {})
            };
            Ok(service)
        }
    }
}

pub struct DNSServiceBrowser {
    pub regtype: String,
    pub domain: Option<String>,
    raw: ffi::DNSServiceRef,
    reply_callback: Box<Fn(u32, i32, &str, &str, &str) -> ()>,
}

impl DNSServiceBrowser {
    unsafe extern "C" fn reply_callback(
        _sd_ref: ffi::DNSServiceRef,
        _flags: ffi::DNSServiceFlags,
        interface_index: u32,
        error_code: ffi::DNSServiceErrorType,
        service_name: *const c_char,
        regtype: *const c_char,
        reply_domain: *const c_char,
        context: *mut c_void,
    ) {
        let context: &mut DNSServiceBrowser = &mut *(context as *mut DNSServiceBrowser);
        // TODO: ensure the C string handling is safe
        let c_str: &CStr = CStr::from_ptr(service_name);
        let service_name: &str = c_str.to_str().unwrap();
        let c_str: &CStr = CStr::from_ptr(regtype);
        let regtype: &str = c_str.to_str().unwrap();
        let c_str: &CStr = CStr::from_ptr(reply_domain);
        let reply_domain: &str = c_str.to_str().unwrap();
        (context.reply_callback)(interface_index, error_code, service_name, regtype, reply_domain);
    }

    fn void_ptr(&mut self) -> *mut c_void {
        self as *mut _ as *mut c_void
    }

    /// Processes a reply from mDNS service, blocking until there is one
    pub fn process_result(&self) -> ffi::DNSServiceErrorType {
        unsafe {
            ffi::DNSServiceProcessResult(self.raw)
        }
    }

    pub fn start<F: 'static>(&mut self, callback: F) -> Result<(), DNSServiceError>
        where F: Fn(u32, i32, &str, &str, &str) -> ()
    {
        // TODO: figure out if we can have non-'static callback
        self.reply_callback = Box::new(callback);
        unsafe {
            let mut domain: *const c_char = ptr::null_mut();
            // TODO: better way to manage CString lifetime here?
            let c_domain: CString;
            if let Some(n) = &self.domain {
                c_domain = CString::new(n.as_str()).map_err(|_| DNSServiceError::InvalidString)?;
                domain = c_domain.as_ptr();
            }
            let service_type = CString::new(self.regtype.as_str()).map_err(|_| DNSServiceError::InvalidString)?;
            ffi::DNSServiceBrowse(&mut self.raw as *mut _, 0, 0, service_type.as_ptr(), domain, Some(DNSServiceBrowser::reply_callback), self.void_ptr());
            Ok(())
        }
    }
}

impl Drop for DNSServiceBrowser {
    fn drop(&mut self) {
        unsafe {
            ffi::DNSServiceRefDeallocate(self.raw);
        }
    }
}