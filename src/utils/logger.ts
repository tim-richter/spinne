import { pino, type Logger } from "pino";

let logger: Logger;

export function initLogger(level = 'info') {
  logger = pino({
    level,
    transport: {
      target: 'pino-pretty',
      options: {
        include: 'level',
      },
    }
  })
}

export function getLogger() {
  if (!logger) {
    initLogger()
  }
  return logger
}
