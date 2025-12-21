import React, { useState, useRef, useEffect } from 'react';
import { Maximize2, Minimize2, Power, RefreshCw, List, X, Maximize } from 'lucide-react';
import { Button } from './ui/button';

interface TerminalActionsProps {
  isConnected: boolean;
  isFullscreen: boolean;
  onConnect: () => void;
  onToggleFullscreen: () => void;
  onRefresh: () => void;
  onListSessions: () => void;
  onTerminateSession: () => void;
  onResizeTerminal: () => void;
  onDownloadFile: () => void;
}

export const TerminalActions: React.FC<TerminalActionsProps> = ({
  isConnected,
  isFullscreen,
  onConnect,
  onToggleFullscreen,
  onRefresh,
  onListSessions,
  onTerminateSession,
  onResizeTerminal,
  onDownloadFile
}) => {
  const [showMoreMenu, setShowMoreMenu] = useState(false);
  const moreMenuRef = useRef<HTMLDivElement>(null);
  
  // 点击外部区域关闭下拉菜单
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (moreMenuRef.current && !moreMenuRef.current.contains(event.target as Node)) {
        setShowMoreMenu(false);
      }
    };
    
    // 添加事件监听器
    document.addEventListener('mousedown', handleClickOutside);
    
    // 清理函数
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, []);

  return (
    <div className="flex items-center space-x-2">
      {/* Main action buttons */}
      <div className="flex items-center space-x-2">
        <Button
          onClick={onConnect}
          variant="default"
          size="sm"
          className={`h-9 w-9 p-0 hover:scale-105 transition-all duration-200 relative overflow-hidden ${isConnected 
            ? 'bg-green-500/30 hover:bg-green-500/40 border-green-500/40 text-white shadow-lg shadow-green-500/30 animate-pulse-slow' 
            : 'bg-gradient-to-br from-blue-500 to-purple-500 hover:from-blue-600 hover:to-purple-600 text-white shadow-lg shadow-blue-500/30 hover:shadow-blue-500/50 animate-pulse'}`}
          title={isConnected ? 'Disconnect' : 'Connect to terminal'}
        >
          <Power size={16} className="relative z-10" />
          {/* Tech glow effect */}
          <div className={`absolute inset-0 opacity-30 pointer-events-none ${isConnected 
            ? 'bg-gradient-to-r from-green-400 via-green-500 to-green-400 animate-glow-shift' 
            : 'bg-gradient-to-r from-blue-400 via-purple-500 to-blue-400 animate-glow-shift'}`}></div>
        </Button>
        
        <Button
          onClick={onToggleFullscreen}
          variant="outline"
          size="sm"
          className="h-9 w-9 p-0 bg-background/80 hover:bg-primary/10 hover:scale-105 transition-all duration-200 relative overflow-hidden border-primary/30 hover:border-primary/50 shadow-md hover:shadow-primary/20"
          title={isFullscreen ? "Exit fullscreen" : "Enter fullscreen"}
        >
          {isFullscreen ? <Minimize2 size={16} className="text-primary relative z-10" /> : <Maximize2 size={16} className="text-primary relative z-10" />}
          {/* Tech glow effect */}
          <div className="absolute inset-0 opacity-20 pointer-events-none bg-gradient-to-r from-primary via-accent to-primary animate-glow-shift"></div>
        </Button>
      </div>
      
      {/* List Sessions Button - Always available */}
      <Button
        onClick={onListSessions}
        variant="outline"
        size="sm"
        className="h-9 w-9 p-0 bg-background/80 hover:bg-primary/10 hover:scale-105 transition-all duration-200 relative overflow-hidden border-primary/30 hover:border-primary/50 shadow-md hover:shadow-primary/20"
        title="List Sessions"
      >
        <List size={16} className="text-primary relative z-10" />
        {/* Tech glow effect */}
        <div className="absolute inset-0 opacity-20 pointer-events-none bg-gradient-to-r from-primary via-accent to-primary animate-glow-shift"></div>
      </Button>
      
      {/* More options dropdown - 作为按钮的直接子元素 */}
      <div className="relative" ref={moreMenuRef}>
        <Button
          variant="outline"
          size="sm"
          className={`h-9 w-9 p-0 hover:scale-105 transition-all duration-200 relative overflow-hidden shadow-sm ${isConnected 
            ? 'bg-primary/20 hover:bg-primary/30 border-primary/30 hover:border-primary/50 shadow-md hover:shadow-primary/20' 
            : 'bg-background/80 opacity-50 cursor-not-allowed'}`}
          title={isConnected ? "More options" : "Connect to enable"}
          disabled={!isConnected}
          onClick={() => setShowMoreMenu(!showMoreMenu)}
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={`relative z-10 ${isConnected ? "text-primary" : "text-muted-foreground"}`}>
            <circle cx="12" cy="12" r="1"></circle>
            <circle cx="19" cy="12" r="1"></circle>
            <circle cx="5" cy="12" r="1"></circle>
          </svg>
          {/* Tech glow effect */}
          {isConnected && (
            <div className="absolute inset-0 opacity-20 pointer-events-none bg-gradient-to-r from-primary via-accent to-primary animate-glow-shift"></div>
          )}
        </Button>
        
        {/* Dropdown content - 直接作为按钮的子元素，使用absolute定位 */}
        {isConnected && showMoreMenu && (
          <div className="absolute right-0 top-full mt-1 bg-card border border-border rounded-lg shadow-lg z-500 py-1 w-48">
            <button
              onClick={() => {
                onResizeTerminal();
                setShowMoreMenu(false);
              }}
              className="w-full flex items-center space-x-2 px-4 py-2 text-sm hover:bg-primary/10 transition-colors"
            >
              <Maximize size={14} className="text-teal-500" />
              <span>Resize Terminal</span>
            </button>
            <button
              onClick={() => {
                onRefresh();
                setShowMoreMenu(false);
              }}
              className="w-full flex items-center space-x-2 px-4 py-2 text-sm hover:bg-primary/10 transition-colors"
            >
              <RefreshCw size={14} className="text-orange-500" />
              <span>Refresh Terminal</span>
            </button>
            <button
              onClick={() => {
                onTerminateSession();
                setShowMoreMenu(false);
              }}
              className="w-full flex items-center space-x-2 px-4 py-2 text-sm text-red-500 hover:bg-red-500/10 transition-colors"
            >
              <X size={14} className="text-red-500" />
              <span>Terminate Session</span>
            </button>
            <button
              onClick={() => {
                onDownloadFile();
                setShowMoreMenu(false);
              }}
              className="w-full flex items-center space-x-2 px-4 py-2 text-sm hover:bg-primary/10 transition-colors"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="text-blue-500">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                <polyline points="7 10 12 15 17 10" />
                <line x1="12" y1="15" x2="12" y2="3" />
              </svg>
              <span>Download File</span>
            </button>
          </div>
        )}
      </div>
    </div>
  );
};