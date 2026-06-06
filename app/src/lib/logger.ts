export interface LogEntry {
  timestamp: string;
  level: 'DEBUG' | 'INFO' | 'WARN' | 'ERROR';
  message: string;
}

type LogListener = (entry: LogEntry) => void;
const listeners = new Set<LogListener>();
let logHistory: LogEntry[] = [];

export const logger = {
  log(level: LogEntry['level'], message: string) {
    const timestamp = new Date().toLocaleTimeString();
    const entry: LogEntry = { timestamp, level, message };
    logHistory.push(entry);
    
    // Cap log history size
    if (logHistory.length > 100) {
      logHistory = logHistory.slice(-100);
    }
    
    listeners.forEach(listener => {
      try {
        listener(entry);
      } catch (e) {
        console.error('Error in log listener:', e);
      }
    });

    const formattedMessage = `[QRT-LOG][${level}][${timestamp}] ${message}`;
    switch (level) {
      case 'ERROR':
        console.error(formattedMessage);
        break;
      case 'WARN':
        console.warn(formattedMessage);
        break;
      default:
        console.log(formattedMessage);
        break;
    }
  },

  debug(message: string) {
    this.log('DEBUG', message);
  },

  info(message: string) {
    this.log('INFO', message);
  },

  warn(message: string) {
    this.log('WARN', message);
  },

  error(message: string) {
    this.log('ERROR', message);
  },

  subscribe(listener: LogListener) {
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  },

  getHistory(): LogEntry[] {
    return [...logHistory];
  },

  clear() {
    logHistory = [];
    listeners.forEach(listener => {
      try {
        listener({ timestamp: new Date().toLocaleTimeString(), level: 'INFO', message: '--- LOG HISTORY CLEARED ---' });
      } catch (e) {
        console.error('Error in log listener:', e);
      }
    });
  }
};
