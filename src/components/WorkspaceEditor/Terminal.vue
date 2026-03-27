<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed, watch } from 'vue';
import { Terminal as XTerm } from '@xterm/xterm';
import { nanoid } from 'nanoid';
import { FitAddon } from '@xterm/addon-fit';
import { useColorMode, useResizeObserver } from '@vueuse/core';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import '@xterm/xterm/css/xterm.css';

const props = defineProps({
  cwd: {
    type: String,
    required: true,
  },
});

const mode = useColorMode();
const terminalRef = ref<HTMLElement>();
let terminal: XTerm | null = null;
let fitAddon: FitAddon | null = null;
let terminalId = nanoid();
let unlisten: (() => void) | null = null;
const showResizeTip = ref(false);
const resizeTipText = ref('');
let resizeTipTimeout: number | null = null;

const terminalTheme = computed(() => {
  const isDark = mode.value === 'dark';
  
  return isDark ? {
    background: '#0f172b',
    foreground: '#d4d4d4',
    viewport: 'transparent',
    cursor: '#d4d4d4',
    black: '#888',
    red: '#cd3131',
    green: '#0dbc79',
    yellow: '#e5e510',
    blue: '#2472c8',
    magenta: '#bc3fbc',
    cyan: '#11a8cd',
    white: '#e5e5e5',
    brightBlack: '#666666',
    brightRed: '#f14c4c',
    brightGreen: '#23d18b',
    brightYellow: '#f5f543',
    brightBlue: '#3b8eea',
    brightMagenta: '#d670d6',
    brightCyan: '#29b8db',
    brightWhite: '#e5e5e5',
  } : {
    background: '#ffffff',
    foreground: '#383a42',
    viewport: 'transparent',
    cursor: '#383a42',
    black: '#383a42',
    red: '#e45649',
    green: '#50a14f',
    yellow: '#c18401',
    blue: '#0184bc',
    magenta: '#a626a4',
    cyan: '#0997b3',
    white: '#fafafa',
    brightBlack: '#4f525e',
    brightRed: '#e06c75',
    brightGreen: '#98c379',
    brightYellow: '#e5c07b',
    brightBlue: '#61afef',
    brightMagenta: '#c678dd',
    brightCyan: '#56b6c2',
    brightWhite: '#ffffff',
  };
});

const sendInput = async (input: string) => {
  try {
    await invoke('terminal_send_input', {
      terminalId,
      input,
    });
  } catch (error) {
    terminal?.writeln(`\r\n\x1b[31mError: ${error}\x1b[0m`);
  }
};

onMounted(async () => {
  if (!terminalRef.value) return;

  terminal = new XTerm({
    cursorBlink: true,
    fontSize: 14,
    fontFamily: 'Menlo, Monaco, Consolas, "Ubuntu Mono", "Courier New", monospace',
    theme: terminalTheme.value,
    convertEol: true,
    scrollback: 10000,
    allowProposedApi: true,
    screenReaderMode: false,
    focusMode: 'click',
  });

  fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);
  terminal.open(terminalRef.value);
  fitAddon.fit();

  // terminal.writeln('\x1b[1;36mWelcome to Raven Terminal\x1b[0m');
  unlisten = await listen(`terminal_output::${terminalId}`, (event: any) => {
    if (!terminal) return;
    
    const { type, data } = event.payload;
    if (type === 'stdout' || type === 'stderr') {
      terminal.write(data);
    } else if (type === 'error') {
      terminal.writeln(`\r\n\x1b[31m${data}\x1b[0m`);
    }
  });

  try {
    await invoke('terminal_create_session', {
      terminalId,
      cwd: props.cwd,
      cols: terminal.cols,
      rows: terminal.rows,
    });
  } catch (error) {
    terminal.writeln(`\x1b[31mFailed to create terminal session: ${error}\x1b[0m`);
  }
  
  terminal.onData((data) => {
    if (!terminal) return;
    sendInput(data);
  });

  // Handle focus events for vim compatibility
  terminal.onFocus(() => {
    // Send focus in sequence (ESC [ I)
    sendInput('\x1b[I');
  });

  terminal.onBlur(() => {
    // Send focus out sequence (ESC [ O)
    sendInput('\x1b[O');
  });

  // Focus the terminal immediately
  terminal.focus();
});

watch(() => mode.value, () => {
  if (terminal) {
    terminal.options.theme = terminalTheme.value;
  }
});

onUnmounted(async () => {
  if (unlisten) {
    unlisten();
  }
  
  try {
    await invoke('terminal_kill_session', {
      terminalId,
    });
  } catch (error) {
    console.error('Failed to kill terminal session:', error);
  }
  
  if (resizeTipTimeout !== null) {
    clearTimeout(resizeTipTimeout);
  }
  
  fitAddon?.dispose();
  terminal?.dispose();
  fitAddon = null;
  terminal = null;
});

useResizeObserver(terminalRef, () => {
  fitAddon?.fit();
  if (terminal) {
    resizeTipText.value = `${terminal.cols} cols × ${terminal.rows} rows`;
    showResizeTip.value = true;
    if (resizeTipTimeout !== null) {
      clearTimeout(resizeTipTimeout);
    }
    resizeTipTimeout = setTimeout(() => {
      showResizeTip.value = false;
    }, 2000);
    invoke('terminal_resize', {
      terminalId,
      cols: terminal.cols,
      rows: terminal.rows,
    });
  }
});
</script>

<template>
  <div ref="terminalRef" class="terminal-container">
    <div v-if="showResizeTip" class="resize-tip">{{ resizeTipText }}</div>
  </div>
</template>

<style lang="scss" scoped>
.terminal-container {
  width: 100%;
  height: 100%;
  padding: 8px;
  overflow: hidden;
  position: relative;
}

.dark .terminal-container {
  background-color: #0f172b;
}

.light .terminal-container {
  background-color: #ffffff;
}

.resize-tip {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  padding: 12px 20px;
  background-color: rgba(0, 0, 0, 0.8);
  color: white;
  border-radius: 6px;
  font-size: 14px;
  font-family: Menlo, Monaco, Consolas, "Ubuntu Mono", "Courier New", monospace;
  pointer-events: none;
  z-index: 10;
}
</style>
