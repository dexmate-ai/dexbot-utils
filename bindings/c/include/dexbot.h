/* Copyright (C) 2026 Dexmate Inc. SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Dexmate-Commercial */
#ifndef DEXBOT_H
#define DEXBOT_H
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* ABI 1. Inputs are borrowed UTF-8 NUL-terminated strings. NULL overlay means none.
 * Roots returned by resolve/profiles/profile_for/parse_urdf/keys are owned: free
 * exactly once with document_free. get/at return borrowed immutable views; NEVER
 * free those views. They and string data remain valid until their root is freed.
 * Concurrent reads are supported; freeing concurrently with reads is invalid.
 * NULL / false indicates failure except is_null. Last error is thread-local,
 * borrowed until the next failed API call on that thread. Invalid non-null
 * pointers, double frees, and use-after-free are caller errors (undefined behavior).
 * size returns zero for empty collections or errors. Do not use it on scalars.
 */
typedef struct dexbot_value dexbot_value;
uint32_t dexbot_abi_version(void);
const char *dexbot_last_error(void);
dexbot_value *dexbot_resolve(const char *source, bool from_file, const char *overlay_yaml);
dexbot_value *dexbot_profiles(void);
dexbot_value *dexbot_profile_for(const char *robot_name);
dexbot_value *dexbot_parse_urdf(const char *source);
void dexbot_document_free(dexbot_value *root);
const dexbot_value *dexbot_get(const dexbot_value *node, const char *key);
const dexbot_value *dexbot_at(const dexbot_value *node, size_t index);
/* null=0, bool=1, number=2, string=3, array=4, object=5; -1 on error. */
int32_t dexbot_type(const dexbot_value *node);
size_t dexbot_size(const dexbot_value *node);
dexbot_value *dexbot_keys(const dexbot_value *node);
/* Returned string is length-delimited, NOT NUL-terminated. */
const char *dexbot_string(const dexbot_value *node, size_t *length);
bool dexbot_number(const dexbot_value *node, double *output);
bool dexbot_boolean(const dexbot_value *node, bool *output);
bool dexbot_is_null(const dexbot_value *node);
/* Owned NUL-terminated JSON; free with string_free. */
char *dexbot_json(const dexbot_value *node);
void dexbot_string_free(char *text);
#ifdef __cplusplus
}
#endif
#endif
