#ifndef ZIP_INTERFACE_H
#define ZIP_INTERFACE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// return codes: 0 == OK
int zip_compress(const uint8_t* data, size_t len, const char* entry_name, uint8_t** out_ptr, size_t* out_len);
int zip_decompress_first(const uint8_t* zip_data, size_t zip_len, uint8_t** out_ptr, size_t* out_len, char** out_name);

// free buffer allocated by the library
void zipffi_free_buffer(void* ptr);

#ifdef __cplusplus
}
#endif

#endif // ZIP_INTERFACE_H
