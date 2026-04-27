use std::fs;
use std::os::unix::io::RawFd;

use crate::device::{DeviceKind, DeviceSpec};
use crate::error::QuadroError;
use crate::protocol::{RawReport, RawStatusReport, RawVirtualSensorsReport};
use crate::services::Logger;
use super::HidrawDevice;

const IOC_WRITE: u32 = 1;
const IOC_READ: u32 = 2;

fn ioctl_ioc(dir: u32, typ: u32, nr: u32, size: u32) -> libc::c_ulong {
    ((dir << 30) | (size << 16) | (typ << 8) | nr) as libc::c_ulong
}

fn hidioc_gfeature(len: usize) -> libc::c_ulong {
    ioctl_ioc(IOC_WRITE | IOC_READ, b'H' as u32, 0x07, len as u32)
}

fn hidioc_sfeature(len: usize) -> libc::c_ulong {
    ioctl_ioc(IOC_WRITE | IOC_READ, b'H' as u32, 0x06, len as u32)
}

fn hidioc_grawinfo() -> libc::c_ulong {
    ioctl_ioc(IOC_READ, b'H' as u32, 0x03, std::mem::size_of::<HidrawDevinfo>() as u32)
}

const USBDEVFS_TYPE: u32 = b'U' as u32;
const USBDEVFS_CLAIMINTERFACE_NR: u32 = 15;
const USBDEVFS_RELEASEINTERFACE_NR: u32 = 16;
const USBDEVFS_BULK_NR: u32 = 2;

const VIRTUAL_SENSORS_BULK_ENDPOINT: u8 = 0x02;
const VIRTUAL_SENSORS_USB_INTERFACE: u32 = 0;
const VIRTUAL_SENSORS_BULK_TIMEOUT_MS: u32 = 2000;

#[repr(C)]
struct UsbdevfsBulktransfer {
    ep: u32,
    len: u32,
    timeout: u32,
    data: *mut libc::c_void,
}

fn usbdevfs_claiminterface() -> libc::c_ulong {
    ioctl_ioc(IOC_READ, USBDEVFS_TYPE, USBDEVFS_CLAIMINTERFACE_NR, 4)
}

fn usbdevfs_releaseinterface() -> libc::c_ulong {
    ioctl_ioc(IOC_READ, USBDEVFS_TYPE, USBDEVFS_RELEASEINTERFACE_NR, 4)
}

fn usbdevfs_bulk() -> libc::c_ulong {
    ioctl_ioc(
        IOC_WRITE | IOC_READ,
        USBDEVFS_TYPE,
        USBDEVFS_BULK_NR,
        std::mem::size_of::<UsbdevfsBulktransfer>() as u32,
    )
}

#[repr(C)]
struct HidrawDevinfo {
    bustype: u32,
    vendor: i16,
    product: i16,
}

const AQUACOMPUTER_VENDOR: u16 = 0x0c70;

pub struct LinuxHidrawDevice {
    fd: RawFd,
    usb: Option<UsbBulkDevice>,
    logger: Box<dyn Logger>,
    spec: DeviceSpec,
}

struct UsbBulkDevice {
    fd: RawFd,
    interface: u32,
}

impl UsbBulkDevice {
    fn open(vendor: u16, product: u16, logger: &dyn Logger) -> Result<Self, QuadroError> {
        let path = find_usb_device_path(vendor, product)?;
        logger.info(&format!("[device] opening USB bulk device at {}", path));
        let c_path = std::ffi::CString::new(path.clone())
            .map_err(|e| QuadroError::InvalidDevicePath(e.to_string()))?;
        let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_RDWR) };
        if fd < 0 {
            let err = std::io::Error::last_os_error();
            logger.error(&format!("[device] failed to open {}: {}", path, err));
            return Err(QuadroError::DeviceOpen { path, source: err });
        }
        let interface: libc::c_uint = VIRTUAL_SENSORS_USB_INTERFACE as libc::c_uint;
        let ret = unsafe {
            libc::ioctl(fd, usbdevfs_claiminterface(), &interface as *const libc::c_uint)
        };
        if ret < 0 {
            let err = std::io::Error::last_os_error();
            unsafe { libc::close(fd); }
            logger.error(&format!("[device] USBDEVFS_CLAIMINTERFACE error: {}", err));
            return Err(QuadroError::Ioctl { operation: "USBDEVFS_CLAIMINTERFACE", source: err });
        }
        logger.info(&format!(
            "[device] claimed USB interface {} on fd={}",
            VIRTUAL_SENSORS_USB_INTERFACE, fd
        ));
        Ok(Self {
            fd,
            interface: VIRTUAL_SENSORS_USB_INTERFACE,
        })
    }

    fn bulk_write(&self, endpoint: u8, data: &[u8], timeout_ms: u32, logger: &dyn Logger) -> Result<usize, QuadroError> {
        let transfer = UsbdevfsBulktransfer {
            ep: endpoint as u32,
            len: data.len() as u32,
            timeout: timeout_ms,
            data: data.as_ptr() as *mut libc::c_void,
        };
        logger.info(&format!(
            "[device] USBDEVFS_BULK: fd={}, ep=0x{:02x}, len={}, timeout={}ms",
            self.fd,
            endpoint,
            data.len(),
            timeout_ms
        ));
        let ret = unsafe {
            libc::ioctl(self.fd, usbdevfs_bulk(), &transfer as *const UsbdevfsBulktransfer)
        };
        if ret < 0 {
            let err = std::io::Error::last_os_error();
            logger.error(&format!("[device] USBDEVFS_BULK error: {}", err));
            return Err(QuadroError::Ioctl { operation: "USBDEVFS_BULK", source: err });
        }
        logger.info(&format!("[device] USBDEVFS_BULK wrote {} bytes", ret));
        Ok(ret as usize)
    }
}

impl Drop for UsbBulkDevice {
    fn drop(&mut self) {
        unsafe {
            libc::ioctl(
                self.fd,
                usbdevfs_releaseinterface(),
                &self.interface as *const libc::c_uint,
            );
            libc::close(self.fd);
        }
    }
}

fn find_usb_device_path(vendor: u16, product: u16) -> Result<String, QuadroError> {
    let entries = fs::read_dir("/sys/bus/usb/devices").map_err(QuadroError::DeviceScan)?;
    for entry in entries {
        let entry = entry.map_err(QuadroError::DeviceScan)?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.contains(':') || name_str.starts_with("usb") {
            // Skip interface endpoints (foo:1.0) and root hubs (usbN)
            if name_str.contains(':') {
                continue;
            }
        }
        let path = entry.path();
        let vid_s = match fs::read_to_string(path.join("idVendor")) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let pid_s = match fs::read_to_string(path.join("idProduct")) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let vid = u16::from_str_radix(vid_s.trim(), 16).unwrap_or(0);
        let pid = u16::from_str_radix(pid_s.trim(), 16).unwrap_or(0);
        if vid != vendor || pid != product {
            continue;
        }
        let busnum_s = fs::read_to_string(path.join("busnum"))
            .map_err(|e| QuadroError::DeviceOpen {
                path: path.join("busnum").to_string_lossy().into_owned(),
                source: e,
            })?;
        let devnum_s = fs::read_to_string(path.join("devnum"))
            .map_err(|e| QuadroError::DeviceOpen {
                path: path.join("devnum").to_string_lossy().into_owned(),
                source: e,
            })?;
        let busnum: u32 = busnum_s.trim().parse().map_err(|_| {
            QuadroError::InvalidDevicePath(format!("invalid busnum: {}", busnum_s.trim()))
        })?;
        let devnum: u32 = devnum_s.trim().parse().map_err(|_| {
            QuadroError::InvalidDevicePath(format!("invalid devnum: {}", devnum_s.trim()))
        })?;
        return Ok(format!("/dev/bus/usb/{:03}/{:03}", busnum, devnum));
    }
    Err(QuadroError::DeviceNotFound)
}

impl LinuxHidrawDevice {
    pub fn open(path: &str, logger: Box<dyn Logger>, spec: DeviceSpec) -> Result<Self, QuadroError> {
        logger.info(&format!("[device] opening {}", path));
        let c_path =
            std::ffi::CString::new(path).map_err(|e| QuadroError::InvalidDevicePath(e.to_string()))?;
        let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_RDWR) };
        if fd < 0 {
            let err = std::io::Error::last_os_error();
            logger.error(&format!("[device] failed to open {}: {}", path, err));
            return Err(QuadroError::DeviceOpen { path: path.to_string(), source: err });
        }
        logger.info(&format!("[device] opened {} -> fd={}", path, fd));
        Ok(Self { fd, usb: None, logger, spec })
    }

    fn ioctl_read(&self, report_id: u8, size: usize) -> Result<Vec<u8>, QuadroError> {
        let mut buf = vec![0u8; size];
        buf[0] = report_id;
        let ioctl_num = hidioc_gfeature(size);
        self.logger.info(&format!(
            "[device] HIDIOCGFEATURE: fd={}, report_id=0x{:02x}, buf_len={}",
            self.fd, report_id, size
        ));
        let ret = unsafe { libc::ioctl(self.fd, ioctl_num, buf.as_mut_ptr()) };
        if ret < 0 {
            let err = std::io::Error::last_os_error();
            self.logger.error(&format!("[device] HIDIOCGFEATURE error: {}", err));
            return Err(QuadroError::Ioctl { operation: "HIDIOCGFEATURE", source: err });
        }
        self.logger.info(&format!("[device] HIDIOCGFEATURE read {} bytes", ret));
        Ok(buf)
    }

    fn ioctl_write(&self, report_id: u8, data: &[u8]) -> Result<usize, QuadroError> {
        let mut buf = data.to_vec();
        buf[0] = report_id;
        let ioctl_num = hidioc_sfeature(buf.len());
        self.logger.info(&format!(
            "[device] HIDIOCSFEATURE: fd={}, report_id=0x{:02x}, buf_len={}",
            self.fd, report_id, buf.len()
        ));
        let ret = unsafe { libc::ioctl(self.fd, ioctl_num, buf.as_ptr()) };
        if ret < 0 {
            let err = std::io::Error::last_os_error();
            self.logger.error(&format!("[device] HIDIOCSFEATURE error: {}", err));
            return Err(QuadroError::Ioctl { operation: "HIDIOCSFEATURE", source: err });
        }
        self.logger.info(&format!("[device] HIDIOCSFEATURE wrote {} bytes", ret));
        Ok(ret as usize)
    }
}

impl HidrawDevice for LinuxHidrawDevice {
    fn read_feature_report(&mut self) -> Result<RawReport, QuadroError> {
        let buf = self.ioctl_read(self.spec.ctrl_report_id, self.spec.ctrl_report_size)
            .map_err(|e| QuadroError::ReportRead(Box::new(e)))?;
        Ok(RawReport::from_bytes(buf, self.spec))
    }

    fn write_feature_report(&mut self, report: &RawReport) -> Result<(), QuadroError> {
        self.ioctl_write(self.spec.ctrl_report_id, report.as_bytes())
            .map_err(|e| QuadroError::ReportWrite(Box::new(e)))?;
        Ok(())
    }

    fn commit(&mut self) -> Result<(), QuadroError> {
        self.ioctl_write(self.spec.secondary_report_id, self.spec.secondary_report)
            .map_err(|e| QuadroError::ReportWrite(Box::new(e)))?;
        Ok(())
    }

    fn write_virtual_sensors(&mut self, report: &RawVirtualSensorsReport) -> Result<(), QuadroError> {
        let data = report.as_bytes();
        if self.usb.is_none() {
            let usb = UsbBulkDevice::open(self.spec.vendor_id, self.spec.product_id, self.logger.as_ref())
                .map_err(|e| QuadroError::ReportWrite(Box::new(e)))?;
            self.usb = Some(usb);
        }
        let usb = self.usb.as_ref().expect("usb is Some after ensure");
        usb.bulk_write(
            VIRTUAL_SENSORS_BULK_ENDPOINT,
            data,
            VIRTUAL_SENSORS_BULK_TIMEOUT_MS,
            self.logger.as_ref(),
        )
        .map_err(|e| QuadroError::ReportWrite(Box::new(e)))?;
        Ok(())
    }

    fn read_status_report(&mut self) -> Result<RawStatusReport, QuadroError> {
        let mut buf = vec![0u8; self.spec.status_report_size];
        self.logger.info(&format!(
            "[device] reading status report ({} bytes)",
            self.spec.status_report_size
        ));
        let ret = unsafe {
            libc::read(self.fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len())
        };
        if ret < 0 {
            let err = std::io::Error::last_os_error();
            self.logger.error(&format!("[device] read error: {}", err));
            return Err(QuadroError::ReportRead(Box::new(
                QuadroError::Ioctl { operation: "read", source: err }
            )));
        }
        self.logger.info(&format!("[device] read {} bytes", ret));
        Ok(RawStatusReport::from_bytes(buf, self.spec))
    }
}

impl Drop for LinuxHidrawDevice {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd); }
    }
}

pub fn find_device(logger: Box<dyn Logger>) -> Result<(LinuxHidrawDevice, DeviceSpec), QuadroError> {
    logger.info("[device] scanning /dev/hidraw* for Aquacomputer devices (QUADRO/OCTO)");
    let entries = std::fs::read_dir("/dev").map_err(QuadroError::DeviceScan)?;

    for entry in entries {
        let entry = entry.map_err(QuadroError::DeviceScan)?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with("hidraw") {
            continue;
        }

        let path = entry.path();
        let path_str = path
            .to_str()
            .ok_or_else(|| QuadroError::InvalidDevicePath("non-utf8 hidraw path".into()))?;

        let temp_spec = DeviceSpec::for_device(DeviceKind::Quadro);
        let device = match LinuxHidrawDevice::open(path_str, Box::new(crate::NullLogger), temp_spec) {
            Ok(d) => d,
            Err(e) => {
                logger.info(&format!("[device] skip {}: {}", path_str, e));
                continue;
            }
        };

        let mut info: HidrawDevinfo = unsafe { std::mem::zeroed() };
        let ret = unsafe {
            libc::ioctl(device.fd, hidioc_grawinfo(), &mut info as *mut HidrawDevinfo)
        };
        if ret < 0 {
            logger.info(&format!("[device] skip {}: HIDIOCGRAWINFO failed", path_str));
            continue;
        }

        logger.info(&format!(
            "[device] {} -> vendor=0x{:04x} product=0x{:04x}",
            path_str, info.vendor as u16, info.product as u16
        ));

        if info.vendor as u16 == AQUACOMPUTER_VENDOR {
            if let Some(spec) = DeviceSpec::from_product_id(info.product as u16) {
                logger.info(&format!("[device] found {} at {}", spec.kind.name(), path_str));
                drop(device);
                let device = LinuxHidrawDevice::open(path_str, logger, spec)?;
                return Ok((device, spec));
            }
        }
    }

    Err(QuadroError::DeviceNotFound)
}

pub fn find_quadro(logger: Box<dyn Logger>) -> Result<LinuxHidrawDevice, QuadroError> {
    find_device(logger).map(|(device, _spec)| device)
}
