import React, { useState } from 'react';
import { UserAvatar, UserInfo } from './UserAvatar';
import { TerminalActions } from './TerminalActions';

interface HeaderProps {
  isConnected: boolean;
  isFullscreen: boolean;
  currentSessionInfo: {
    sessionId: string;
    shellType: string;
    terminalSize: { columns: number; rows: number };
  };
  protocol: 'websocket' | 'webtransport' | 'auto';
  onProtocolChange: (protocol: 'websocket' | 'webtransport' | 'auto') => void;
  onConnect: () => void;
  onToggleFullscreen: () => void;
  onRefresh: () => void;
  onListUserSessions: () => void;
  onListAllSessions: () => void;
  onTerminateSession: () => void;
  onResizeTerminal: () => void;
  onDownloadFile: () => void;
}

export const Header: React.FC<HeaderProps> = ({
  isConnected,
  isFullscreen,
  currentSessionInfo,
  protocol,
  onProtocolChange,
  onConnect,
  onToggleFullscreen,
  onRefresh,
  onListUserSessions,
  onListAllSessions,
  onTerminateSession,
  onResizeTerminal,
  onDownloadFile
}) => {
  // 模拟用户信息
  const [userInfo] = useState<UserInfo>({
    id: 'user-1',
    name: 'Long Wang',
    avatar: 'https://ui-avatars.com/api/?name=Long+Wang&background=random',
    sessionLimit: 5,
    activeSessions: 1
  });

  return (
    <header className="glass border-b border-border/50 px-4 py-3 relative z-10">
      <div className="flex items-center justify-between max-w-7xl mx-auto">
        <div className="flex items-center space-x-4">
          {/* Logo and Status */}
          <div className="flex items-center space-x-3">
            <div className={`w-3 h-3 rounded-full relative ${isConnected ? 'status-connected glow-success' : 'status-disconnected'}`}>
              <div className={`absolute inset-0 rounded-full animate-ping ${isConnected ? 'bg-green-400' : 'bg-red-400'} opacity-75`}></div>
            </div>
            <h1 className="text-lg font-semibold tracking-tight bg-gradient-to-r from-primary to-accent bg-clip-text text-transparent">
              Waylon Terminal
            </h1>
          </div>
          
          {/* Protocol selection */}
          <div className="flex items-center space-x-2 ml-4 pl-4 border-l border-border">
            <span className="text-sm text-muted-foreground">PROTOCOL:</span>
            <div className="relative">
              <select 
                className="appearance-none px-4 py-2 bg-background/80 border border-primary/30 rounded-lg text-sm text-primary focus:border-primary focus:outline-none focus:ring-2 focus:ring-primary/30 transition-all duration-200 shadow-sm hover:border-primary/50 disabled:opacity-70 disabled:cursor-not-allowed pr-8"
                value={protocol}
                onChange={(e) => onProtocolChange(e.target.value as 'websocket' | 'webtransport' | 'auto')}
                disabled={isConnected}
              >
                <option value="auto" className="bg-background text-foreground">Auto</option>
                <option value="websocket" className="bg-background text-foreground">WebSocket</option>
                <option value="webtransport" className="bg-background text-foreground">WebTransport</option>
              </select>
              {/* Custom dropdown arrow */}
              <div className="absolute inset-y-0 right-0 flex items-center pr-2 pointer-events-none">
                <svg className="h-4 w-4 text-primary/70" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                </svg>
              </div>
            </div>
          </div>
          
          {/* Session info display */}
          {isConnected && (
            <div className="flex items-center space-x-4 ml-4 pl-4 border-l border-border">
              <div className="flex flex-col text-sm">
                <span className="text-muted-foreground">SHELL: {currentSessionInfo.shellType}</span>
              </div>
              <div className="flex flex-col text-sm">
                <span className="text-primary font-mono">SIZE: {currentSessionInfo.terminalSize.columns}×{currentSessionInfo.terminalSize.rows}</span>
              </div>
            </div>
          )}
        </div>
        
        <div className="flex items-center space-x-4">
          {/* Terminal Actions Component */}
          <TerminalActions
            isConnected={isConnected}
            isFullscreen={isFullscreen}
            onConnect={onConnect}
            onToggleFullscreen={onToggleFullscreen}
            onRefresh={onRefresh}
            onListSessions={onListAllSessions}
            onTerminateSession={onTerminateSession}
            onResizeTerminal={onResizeTerminal}
            onDownloadFile={onDownloadFile}
          />
          
          {/* User Avatar Component */}
          <UserAvatar 
            userInfo={userInfo} 
            onListSessions={onListUserSessions} 
          />
        </div>
      </div>
    </header>
  );
};