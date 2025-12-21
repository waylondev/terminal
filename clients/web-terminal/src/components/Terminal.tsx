import React, { useEffect, useRef, useState, forwardRef, useImperativeHandle } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { TerminalService, initializeTerminal, terminalUtils } from '../services/terminalService';
import { downloadFile as apiDownloadFile } from '../services/terminalApi';

interface TerminalComponentProps {
  className?: string;
  protocol?: 'websocket' | 'webtransport' | 'auto';
  onConnectionStatusChange?: (connected: boolean, sessionInfo?: { sessionId: string; shellType: string; terminalSize: { columns: number; rows: number } }) => void;
  ref?: React.Ref<any>;
}

const TerminalComponent = forwardRef<any, TerminalComponentProps>(({ className, protocol = 'auto', onConnectionStatusChange }, ref) => {
  const terminalRef = useRef<HTMLDivElement>(null);
  const terminal = useRef<Terminal | null>(null);
  const fitAddon = useRef<FitAddon | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const [sessionId, setSessionId] = useState<string>('');
  const sessionIdRef = useRef<string>(''); // Ref to track current sessionId
  const terminalService = useRef<TerminalService | null>(null);
  const isInitialized = useRef(false);

  // Expose methods to parent component
  useImperativeHandle(ref, () => ({
    connect: connectTerminal,
    disconnect: () => {
      terminalService.current?.disconnect();
    },
    send: (data: string) => {
      terminalService.current?.send(data);
    },
    resize: handleResize,
    interrupt: handleInterrupt,
    terminate: handleTerminate,
    clear: () => {
      if (terminal.current) {
        terminal.current.clear();
        // 清屏后不自动添加提示符，避免重复的$符号
        // 提示符会在用户输入时由终端自动显示
      }
    },
    downloadFile: (filePath: string) => {
      console.log('🎯 Terminal.downloadFile called with filePath:', filePath);
      const currentSessionId = sessionIdRef.current;
      console.log('🎯 currentSessionId:', currentSessionId);
      if (!currentSessionId) {
        console.error('❌ Failed to download file: No session ID available');
        alert('Failed to download file: No session ID available');
        return;
      }
      console.log('🎯 Calling apiDownloadFile with sessionId:', currentSessionId, 'and filePath:', filePath);
      apiDownloadFile(currentSessionId, filePath).catch(error => {
        console.error('❌ Failed to download file:', error);
        alert(`Failed to download file: ${error.message}`);
      });
    },
    updateSettings: (settings: any) => {
      if (terminal.current) {
        // Update font size
        if (settings.fontSize) {
          terminal.current.options.fontSize = settings.fontSize;
        }
        
        // Update font family
        if (settings.fontFamily) {
          terminal.current.options.fontFamily = settings.fontFamily;
        }
        
        // Update theme
        if (settings.theme) {
          switch (settings.theme) {
            case 'light':
              terminal.current.options.theme = {
                background: '#ffffff',
                foreground: '#000000',
                cursor: '#000000'
              };
              break;
            case 'high-contrast':
              terminal.current.options.theme = {
                background: '#000000',
                foreground: '#ffffff',
                cursor: '#ffffff'
              };
              break;
            case 'dark':
            default:
              terminal.current.options.theme = {
                background: '#1e1e1e',
                foreground: '#cccccc',
                cursor: '#ffffff'
              };
              break;
          }
        }
        
        // Update cursor style
        if (settings.cursorStyle) {
          terminal.current.options.cursorStyle = settings.cursorStyle as any;
        }
        
        // Update scrollback
        if (settings.scrollback) {
          terminal.current.options.scrollback = settings.scrollback;
        }
        
        // Update auto wrap - removed due to API changes in xterm.js
        // if (settings.autoWrap !== undefined) {
        //   terminal.current.options.wrap = settings.autoWrap;
        // }
        
        // Refresh terminal to apply changes
        terminal.current.refresh(0, terminal.current.rows - 1);
      }
    },
    isConnected: () => isConnected,
    getSessionId: () => sessionId
  }));

  // Connection status change handler
  const handleConnectionStatusChange = (connected: boolean, sessionInfo?: {
    sessionId: string;
    shellType: string;
    terminalSize: { columns: number; rows: number };
  }) => {
    setIsConnected(connected);
    if (connected && sessionInfo) {
      setSessionId(sessionInfo.sessionId);
      sessionIdRef.current = sessionInfo.sessionId; // Update ref as well
    } else {
      setSessionId('');
      sessionIdRef.current = ''; // Update ref as well
    }
    onConnectionStatusChange?.(connected, sessionInfo);
  };

  // Connect terminal
  const connectTerminal = async () => {
    if (!terminal.current) {
      console.error('❌ Terminal not initialized');
      return;
    }

    // 使用fitAddon获取当前终端的实际尺寸
    fitAddon.current?.fit();
    
    // 获取终端尺寸
    const columns = terminal.current?.cols || 80;
    const rows = terminal.current?.rows || 24;

    // 创建或使用现有的终端服务实例
    if (!terminalService.current) {
      terminalService.current = new TerminalService(protocol, handleConnectionStatusChange);
    }

    // 连接终端
    await terminalService.current.connect(terminal.current, columns, rows);
  };
  
  // Resize terminal
  const handleResize = async (columns: number, rows: number) => {
    await terminalService.current?.resize(columns, rows);
    
    // 更新xterm.js尺寸
    if (terminal.current) {
      terminal.current.resize(columns, rows);
    }
  };
  
  // Interrupt terminal (send Ctrl+C signal)
  const handleInterrupt = async () => {
    await terminalService.current?.interrupt();
    
    // 在终端中显示中断提示
    if (terminal.current) {
      terminal.current.write('\r\n^C\r\n');
    }
  };
  
  // Terminate session
  const handleTerminate = async (reason?: string) => {
    await terminalService.current?.terminate(reason);
  };

  // Initialize terminal
  useEffect(() => {
    if (!terminalRef.current || isInitialized.current) return;

    console.log('🎯 Initializing xterm.js terminal...');
    
    // 初始化终端
    const { terminal: initializedTerminal, fitAddon: initializedFitAddon } = initializeTerminal(
      terminalRef,
      (data) => {
        console.log('⌨️ Terminal input:', data);
        terminalService.current?.send(data);
      }
    );

    terminal.current = initializedTerminal;
    fitAddon.current = initializedFitAddon;

    // 使用fitAddon获取实际终端尺寸
    setTimeout(() => {
      // 使用fitAddon让终端自动适应容器大小，获取实际尺寸
      fitAddon.current?.fit();
      
      // 窗口大小改变时，重新调整终端大小
      const handleResize = () => {
        terminalUtils.handleWindowResize(fitAddon.current!);
      };
      
      window.addEventListener('resize', handleResize);
      
      // Cleanup function
      return () => {
        window.removeEventListener('resize', handleResize);
      };
    }, 100);

    isInitialized.current = true;
    console.log('✅ Terminal initialized successfully');
    
    // Display welcome message
    terminalUtils.showWelcomeMessage(terminal.current);

  }, []);

  return (
    <div className={`relative ${className}`}>
      {/* Connection status indicator */}
      <div className="absolute top-2 right-2 z-10 flex items-center space-x-2">
        <div 
          className={`w-3 h-3 rounded-full ${
            isConnected ? 'bg-green-500' : 'bg-red-500'
          }`}
        />
        <span className="text-xs text-white bg-black bg-opacity-50 px-2 py-1 rounded">
          {isConnected ? 'Connected' : 'Disconnected'}
        </span>
        {sessionId && (
          <span className="text-xs text-gray-300 bg-black bg-opacity-50 px-2 py-1 rounded">
            ID: {sessionId}
          </span>
        )}
      </div>
      
      <div 
        ref={terminalRef} 
        className="w-full bg-slate-900"
      />
    </div>
  );
});

export { TerminalComponent };