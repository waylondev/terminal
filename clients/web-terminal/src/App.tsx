import { useState, useRef } from 'react';
import { TerminalComponent } from './components/Terminal';
import { Header } from './components/Header';
import { SessionListModal } from './components/SessionListModal';
import { ResizeModal } from './components/ResizeModal';
import { DownloadModal } from './components/DownloadModal';
import { listSessions } from './services/terminalApi';

function App() {
  const terminalRef = useRef<any>(null);
  const [isConnected, setIsConnected] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);
  // 协议选择状态
  const [protocol, setProtocol] = useState<'websocket' | 'webtransport' | 'auto'>('websocket');
  
  // 会话信息状态 - 使用与后端一致的默认尺寸
  const [currentSessionInfo, setCurrentSessionInfo] = useState<{
    sessionId: string;
    shellType: string;
    terminalSize: { columns: number; rows: number };
  }>({
    sessionId: '',
    shellType: '',
    terminalSize: { columns: 80, rows: 24 } // 与后端默认尺寸保持一致
  });

  // Download Modal state and handlers
  const [showDownloadModal, setShowDownloadModal] = useState(false);

  const toggleFullscreen = () => {
    if (!document.fullscreenElement) {
      document.documentElement.requestFullscreen();
      setIsFullscreen(true);
    } else {
      if (document.exitFullscreen) {
        document.exitFullscreen();
        setIsFullscreen(false);
      }
    }
  };

  const handleConnect = () => {
    if (isConnected) {
      // Disconnect
      if (terminalRef.current && terminalRef.current.disconnect) {
        terminalRef.current.disconnect();
      }
      setIsConnected(false);
    } else {
      // Connect
      if (terminalRef.current && terminalRef.current.connect) {
        terminalRef.current.connect();
      }
      setIsConnected(true);
    }
  };

  const handleConnectionStatusChange = (connected: boolean, sessionInfo?: {
    sessionId: string;
    shellType: string;
    terminalSize: { columns: number; rows: number }
  }) => {
    setIsConnected(connected);
    
    if (connected && sessionInfo) {
      // 更新会话信息
      setCurrentSessionInfo({
        sessionId: sessionInfo.sessionId,
        shellType: sessionInfo.shellType,
        terminalSize: sessionInfo.terminalSize
      });
      
      // 根据终端尺寸动态调整布局
      if (sessionInfo.terminalSize) {
        console.log('📏 Terminal size updated:', `${sessionInfo.terminalSize.columns}×${sessionInfo.terminalSize.rows}`);
        // 这里可以根据实际尺寸调整布局，比如设置合适的容器高度
      }
    } else {
      // 断开连接时重置会话信息
      setCurrentSessionInfo({
        sessionId: '',
        shellType: '',
        terminalSize: { columns: 120, rows: 30 }
      });
    }
  };

  // Control panel API operations
  const handleRefresh = () => {
    if (terminalRef.current && terminalRef.current.connect) {
      terminalRef.current.disconnect();
      setTimeout(() => {
        terminalRef.current.connect();
      }, 500);
    }
  };

  // Session List Modal state and handlers
  const [showSessionList, setShowSessionList] = useState(false);
  const [sessionList, setSessionList] = useState<any[]>([]);
  const [sessionListTitle, setSessionListTitle] = useState('Sessions');

  // Get or generate user ID
  const getUserId = () => {
    let userId = localStorage.getItem('terminal_user_id');
    if (!userId) {
      // Generate user ID in format required by backend: usr_ + 12 lowercase hex characters
      const hexChars = 'abcdef0123456789';
      let hexId = '';
      for (let i = 0; i < 12; i++) {
        hexId += hexChars.charAt(Math.floor(Math.random() * hexChars.length));
      }
      userId = 'usr_' + hexId.toLowerCase();
      localStorage.setItem('terminal_user_id', userId);
    }
    return userId;
  };

  // Handle listing all sessions (global)
  const handleListAllSessions = async () => {
    try {
      const data = await listSessions();
      console.log('All sessions:', data);
      setSessionList(data.sessions);
      setSessionListTitle('Global Sessions');
      setShowSessionList(true);
    } catch (error) {
      console.error('Failed to list all sessions:', error);
    }
  };

  // Handle listing user's sessions
  const handleListUserSessions = async () => {
    try {
      const userId = getUserId();
      const data = await listSessions(userId);
      console.log('User sessions:', data);
      setSessionList(data.sessions);
      setSessionListTitle('My Sessions');
      setShowSessionList(true);
    } catch (error) {
      console.error('Failed to list user sessions:', error);
    }
  };

  const handleTerminateSession = () => {
    if (terminalRef.current && terminalRef.current.terminate) {
      terminalRef.current.terminate('USER_REQUESTED');
    }
  };

  const handleDownloadFile = () => {
    setShowDownloadModal(true);
  };

  const handleDownloadFileWithPath = (filePath: string) => {
    if (terminalRef.current && terminalRef.current.downloadFile) {
      terminalRef.current.downloadFile(filePath);
    }
  };

  // Resize Modal state and handlers
  const [showResizeModal, setShowResizeModal] = useState(false);

  const handleResizeTerminal = () => {
    setShowResizeModal(true);
  };

  const handleApplyResize = (columns: number, rows: number) => {
    if (terminalRef.current && terminalRef.current.resize) {
      terminalRef.current.resize(columns, rows);
    }
  };

  return (
    <div className="h-screen flex flex-col bg-gradient-to-br from-tech-bg-darker via-tech-bg-dark to-tech-secondary text-foreground font-sans overflow-hidden bg-grid-pattern relative">
      {/* Tech Background Elements */}
      <div className="absolute inset-0 bg-gradient-to-br from-primary/5 via-accent/5 to-primary/5 pointer-events-none"></div>
      <div className="absolute inset-0 bg-gradient-to-br from-tech-bg-darker via-transparent to-tech-bg-darker pointer-events-none"></div>
      {/* Header Component */}
      <Header
        isConnected={isConnected}
        isFullscreen={isFullscreen}
        currentSessionInfo={currentSessionInfo}
        protocol={protocol}
        onProtocolChange={setProtocol}
        onConnect={handleConnect}
        onToggleFullscreen={toggleFullscreen}
        onRefresh={handleRefresh}
        onListUserSessions={handleListUserSessions}
        onListAllSessions={handleListAllSessions}
        onTerminateSession={handleTerminateSession}
        onResizeTerminal={handleResizeTerminal}
        onDownloadFile={handleDownloadFile}
      />

      {/* Session List Modal Component */}
      <SessionListModal
        isOpen={showSessionList}
        sessions={sessionList}
        currentSessionId={currentSessionInfo.sessionId}
        title={sessionListTitle}
        onClose={() => setShowSessionList(false)}
      />
      
      {/* Resize Modal Component */}
      <ResizeModal
        isOpen={showResizeModal}
        currentSize={currentSessionInfo.terminalSize}
        onClose={() => setShowResizeModal(false)}
        onApplyResize={handleApplyResize}
      />
      
      {/* Download Modal Component */}
      <DownloadModal
        isOpen={showDownloadModal}
        onClose={() => setShowDownloadModal(false)}
        onDownload={handleDownloadFileWithPath}
      />

      {/* Main Content - Full width with minimal padding */}
      <div className="flex-1 flex flex-col overflow-auto">
        {/* Terminal area that can grow with content */}
        <div className="p-4">
          {/* Terminal Container with enhanced border effect - full width */}
          <div className="relative w-full">
            {/* Enhanced tech background effects */}
            <div className="absolute inset-0 bg-gradient-to-br from-primary/20 to-accent/20 opacity-40 pointer-events-none"></div>
            
            {/* Sharp tech glow overlay - no blur */}
            <div className="absolute inset-0 bg-gradient-to-br from-primary/15 via-accent/15 to-primary/15 pointer-events-none"></div>
            
            {/* Digital circuit pattern overlay */}
            <div className="absolute inset-0 bg-[url('data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iNjAiIGhlaWdodD0iNjAiIHZpZXdCb3g9IjAgMCA2MCA2MCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48ZyBmaWxsPSJub25lIiBmaWxsLXJ1bGU9ImV2ZW5vZGQiPjxwYXRoIGQ9Ik0zNiAxOGMtMi4yMSAwLTQgMS43OS00IDRzMS43OSA0IDQgNGM1LjUyMyAwIDEwLTMuOTc3IDEwLTEwcy0zLjk3Ny0xMC0xMC0xMGMtMi4yMSAwLTQgMS43OS00IDRzMS43OSA0IDQgNHoiIGZpbGw9IiMwMDgwZmYiIGZpbGwtb3BhY2l0eT0iMC4xIi8+PHBhdGggZD0iTTMwIDE4YzIuMjEgMCA0LTEuNzkgNC00cy0xLjc5LTQtNC00Yy01LjUyMyAwLTEwIDMuOTc3LTEwIDEwczMuOTc3IDEwIDEwIDEwYzIuMjEgMCA0LTEuNzkgNC00cy0xLjc5LTQtNC00eiIgZmlsbD0iIzAwODBmZiIgZmlsbC1vcGFjaXR5PSIwLjEiLz48cGF0aCBkPSJNMTggMThjMi4yMSAwIDQtMS43OSA0LTRzLTEuNzktNC00LTRjLTUuNTIzIDAtMTAgMy45NzctMTAgMTBzMy45NzcgMTAgMTAgMTBjMi4yMSAwIDQtMS43OSA0LTRzLTEuNzktNC00LTR6IiBmaWxsPSIjMDA4MGZmIiBmaWxsLW9wYWNpdHk9IjAuMSIvPjxwYXRoIGQ9Ik00OCAxOGMyLjIxIDAgNC0xLjc5IDQtNHMtMS43OS00LTQtNGMtNS41MjMgMC0xMCAzLjk3Ny0xMCAxMHMzLjk3NyAxMCAxMCAxMGMzLjMxNCAwIDYtMi42ODYgNi02cy0yLjY4Ni02LTYtNnptLTEyIDBjMi4yMSAwIDQtMS43OSA0LTRzLTEuNzktNC00LTRzLTQgMS43OS00IDQgMS43OSA0IDQgNHoiIGZpbGw9IiMwMDgwZmYiIGZpbGwtb3BhY2l0eT0iMC4xIi8+PC9nPjwvc3ZnPg==')] opacity-20 pointer-events-none"></div>
            
            {/* Monitor frame - enhanced border effect with tech elements */}
            <div className="relative bg-card/95 backdrop-blur-xl border-2 border-primary/40 shadow-2xl">
              {/* Modern tech elements - dynamic gradient border */}
              <div className="absolute inset-0 bg-gradient-to-br from-primary/20 via-accent/20 to-primary/20 opacity-40 animate-gradientShift pointer-events-none"></div>
              
              {/* Enhanced glowing edges with pulse effect */}
              <div className="absolute bottom-0 left-0 w-full h-1 bg-gradient-to-r from-primary via-accent to-primary opacity-90 animate-pulse-slow pointer-events-none"></div>
              <div className="absolute top-0 left-0 h-full w-1 bg-gradient-to-b from-primary via-accent to-primary opacity-90 animate-pulse-slow pointer-events-none"></div>
              <div className="absolute top-0 right-0 h-full w-1 bg-gradient-to-b from-primary via-accent to-primary opacity-90 animate-pulse-slow pointer-events-none"></div>
              
              {/* Corner accents - tech style */}
              <div className="absolute top-0 left-0 w-6 h-6 border-t-2 border-l-2 border-primary opacity-90 pointer-events-none"></div>
              <div className="absolute top-0 right-0 w-6 h-6 border-t-2 border-r-2 border-primary opacity-90 pointer-events-none"></div>
              <div className="absolute bottom-0 left-0 w-6 h-6 border-b-2 border-l-2 border-primary opacity-90 pointer-events-none"></div>
              <div className="absolute bottom-0 right-0 w-6 h-6 border-b-2 border-r-2 border-primary opacity-90 pointer-events-none"></div>
              
              {/* Scan line effect with digital distortion */}
              <div className="absolute inset-0 bg-gradient-to-b from-transparent via-black/8 to-transparent pointer-events-none"></div>
              <div className="absolute inset-0 bg-[linear-gradient(0deg,rgba(0,0,0,0.12)0%,rgba(0,0,0,0)100%)] bg-[length:100%_3px] pointer-events-none"></div>
              
              {/* Tech glow pulses - corner effects */}
              <div className="absolute top-0 left-0 w-12 h-12 bg-primary/30 rounded-full blur-xl animate-pulse pointer-events-none"></div>
              <div className="absolute top-0 right-0 w-12 h-12 bg-accent/30 rounded-full blur-xl animate-pulse delay-300 pointer-events-none"></div>
              <div className="absolute bottom-0 left-0 w-12 h-12 bg-accent/30 rounded-full blur-xl animate-pulse delay-600 pointer-events-none"></div>
              <div className="absolute bottom-0 right-0 w-12 h-12 bg-primary/30 rounded-full blur-xl animate-pulse delay-900 pointer-events-none"></div>
              
              {/* Digital data stream effects */}
              <div className="absolute bottom-0 left-0 w-full h-3 bg-gradient-to-r from-transparent via-accent/50 to-transparent animate-data-flow-reverse pointer-events-none"></div>
              
              {/* Vertical data streams on sides */}
              <div className="absolute top-0 left-0 h-full w-3 bg-gradient-to-b from-transparent via-primary/40 to-transparent animate-data-stream pointer-events-none"></div>
              <div className="absolute top-0 right-0 h-full w-3 bg-gradient-to-b from-transparent via-accent/40 to-transparent animate-data-stream-reverse pointer-events-none"></div>
              
              {/* Monitor screen with padding to avoid content clipping */}
              <div className="relative p-4">
                <TerminalComponent 
                  ref={terminalRef}
                  className="w-full"
                  protocol={protocol}
                  onConnectionStatusChange={handleConnectionStatusChange}
                />
              </div>
            </div>
          </div>
        </div>
        
        {/* Author and repository info with glass effect */}
        <div className="glass p-3 flex items-center justify-between text-sm text-muted-foreground relative overflow-hidden rounded-xl m-4 mt-0">
          {/* Modern decorative elements */}
          <div className="absolute inset-0 bg-gradient-to-r from-primary/10 via-accent/10 to-primary/10 opacity-50 pointer-events-none"></div>
          <div className="absolute -top-1 left-1/2 w-24 h-1 bg-gradient-to-r from-transparent via-primary/50 to-transparent animate-pulse-slow pointer-events-none"></div>
          
          {/* Tech grid overlay */}
          <div className="absolute inset-0 bg-grid-pattern opacity-20 pointer-events-none"></div>
          
          {/* Author info */}
          <div className="flex items-center space-x-2 relative z-10">
            <svg className="w-4 h-4 text-primary/70" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
            </svg>
            <span className="font-mono">Author: </span>
            <a 
              href="https://github.com/waylondev" 
              target="_blank" 
              rel="noopener noreferrer" 
              className="font-mono hover:text-primary transition-colors bg-gradient-to-r from-primary to-accent bg-clip-text text-transparent"
            >
              long.wang
            </a>
          </div>
          
          {/* Repository info */}
          <div className="flex items-center space-x-2 relative z-10">
            <a 
              href="https://github.com/waylondev/terminal" 
              target="_blank" 
              rel="noopener noreferrer" 
              className="flex items-center space-x-1 hover:text-primary transition-colors"
            >
              <svg className="w-4 h-4 text-primary/70" fill="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                <path fillRule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" clipRule="evenodd" />
              </svg>
              <span className="font-mono bg-gradient-to-r from-primary to-accent bg-clip-text text-transparent">github.com/waylondev/terminal</span>
            </a>
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;