package zip_ffi

/*
	#cgo CFLAGS: -I${SRCDIR}/include
	#cgo LDFLAGS: -lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp -L${SRCDIR}/bin -lzip
	#include <stdlib.h>
	#include <zip_interface.h>
*/
import "C"
import (
	"errors"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"unsafe"
)

func init() {
	// 动态库最终路径
	var libFile string
	switch runtime.GOOS {
	case "windows":
		libFile = "bin/zip.dll"
	case "darwin":
		libFile = "bin/libzip.dylib"
	default:
		libFile = "bin/libzip.so"
	}

	// 如果库不存在，则编译 Rust 并复制到 bin/
	if _, err := os.Stat(libFile); os.IsNotExist(err) {
		// Rust 源码目录（Cargo.toml 所在目录）
		rustDir := "../" // 根据你的目录结构调整
		buildCmd := exec.Command("cargo", "build", "--release")
		buildCmd.Dir = rustDir
		buildCmd.Stdout = os.Stdout
		buildCmd.Stderr = os.Stderr
		if err := buildCmd.Run(); err != nil {
			panic("Failed to build Rust library: " + err.Error())
		}

		// 源文件路径（默认 target/release/）
		var srcLib string
		switch runtime.GOOS {
		case "windows":
			srcLib = filepath.Join(rustDir, "target", "release", "zip.dll")
		case "darwin":
			srcLib = filepath.Join(rustDir, "target", "release", "libzip.dylib")
		default:
			srcLib = filepath.Join(rustDir, "target", "release", "libzip.so")
		}

		// 确保 bin 目录存在
		_ = os.MkdirAll("bin", 0755)

		// 复制库到 bin/
		input, err := os.ReadFile(srcLib)
		if err != nil {
			panic("Failed to read Rust library: " + err.Error())
		}
		if err := os.WriteFile(libFile, input, 0644); err != nil {
			panic("Failed to write library to bin/: " + err.Error())
		}
	}
}

// Compress 将 data 压成一个 zip（包含一个 entryName 文件），返回 zip bytes 或 error。
func Compress(data []byte, entryName string) ([]byte, error) {
	if len(data) == 0 {
		return nil, errors.New("zip compress: input empty")
	}
	cEntry := C.CString(entryName)
	defer C.free(unsafe.Pointer(cEntry))

	var outPtr *C.uint8_t
	var outLen C.size_t

	// call
	ret := C.zip_compress((*C.uint8_t)(unsafe.Pointer(&data[0])), C.size_t(len(data)), cEntry, &outPtr, &outLen)
	if ret != 0 {
		return nil, errors.New("zip compress failed")
	}
	if outPtr == nil || outLen == 0 {
		return []byte{}, nil
	}
	// copy to Go slice
	goBytes := C.GoBytes(unsafe.Pointer(outPtr), C.int(outLen))
	// free C buffer
	C.zipffi_free_buffer(unsafe.Pointer(outPtr))
	return goBytes, nil
}

// Decompress 解压 zipData，返回第一个文件内容、文件名（string）、error
func Decompress(zipData []byte) ([]byte, string, error) {
	if len(zipData) == 0 {
		return nil, "", errors.New("zip decompress: input empty")
	}
	var outPtr *C.uint8_t
	var outLen C.size_t
	var outName *C.char

	ret := C.zip_decompress_first((*C.uint8_t)(unsafe.Pointer(&zipData[0])), C.size_t(len(zipData)), &outPtr, &outLen, &outName)
	if ret != 0 {
		return nil, "", errors.New("zip decompress first failed")
	}
	var name string
	if outName != nil {
		name = C.GoString(outName)
		C.zipffi_free_buffer(unsafe.Pointer(outName))
	} else {
		name = ""
	}
	var data []byte
	if outPtr != nil && outLen > 0 {
		data = C.GoBytes(unsafe.Pointer(outPtr), C.int(outLen))
		C.zipffi_free_buffer(unsafe.Pointer(outPtr))
	} else {
		data = []byte{}
	}
	return data, name, nil
}
