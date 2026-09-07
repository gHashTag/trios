/**
 * TRIOS Logger — Simple console-based logger
 */

type LogLevel = 'debug' | 'info' | 'warn' | 'error'

interface LogContext {
  [key: string]: any
}

class Logger {
  private level: LogLevel = 'info'

  setLevel(level: LogLevel) {
    this.level = level
  }

  private shouldLog(level: LogLevel): boolean {
    const levels: LogLevel[] = ['debug', 'info', 'warn', 'error']
    return levels.indexOf(level) >= levels.indexOf(this.level)
  }

  debug(message: string, context?: LogContext) {
    if (!this.shouldLog('debug')) return
    console.log(`[DEBUG] ${message}`, context || '')
  }

  info(message: string, context?: LogContext) {
    if (!this.shouldLog('info')) return
    console.log(`[INFO] ${message}`, context || '')
  }

  warn(message: string, context?: LogContext) {
    if (!this.shouldLog('warn')) return
    console.warn(`[WARN] ${message}`, context || '')
  }

  error(message: string, context?: LogContext) {
    if (!this.shouldLog('error')) return
    console.error(`[ERROR] ${message}`, context || '')
  }
}

export const logger = new Logger()
