#pragma once

#include "../argp.h"
#include "../config.h"

/**
 * @brief Initialise logging
 */
void init_logs(Args* args);

/**
 * @brief Finalise logging
 */
void fini_logs(void);

/**
 * @brief Write a warning to stderr
 *
 * @param format Warning format (printf)
 * @param ... Possible printf arguments
 */
void log_warn(const char* restrict format, ...) __attribute__((format(printf, 1, 2)));
/**
 * @brief Write an error stderr
 *
 * @param format Error format (printf)
 * @param ... Possible printf arguments
 */
void log_err(const char* restrict format, ...) __attribute__((cold)) __attribute__((format(printf, 1, 2)));
/**
 * @brief Write information to stderr
 *
 * @param format Information format (printf)
 * @param ... Possible printf arguments
 */
void log_info(const char* restrict format, ...) __attribute__((format(printf, 1, 2)));
/**
 * @brief Write a success message to stderr
 *
 * @param format Success message format (printf)
 * @param ... Possible printf arguments
 */
void log_succ(const char* restrict format, ...) __attribute__((format(printf, 1, 2)));
