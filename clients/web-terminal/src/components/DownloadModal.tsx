import React, { useState, useEffect } from 'react';

interface DownloadModalProps {
  isOpen: boolean;
  onClose: () => void;
  onDownload: (filePath: string) => void;
}

export const DownloadModal: React.FC<DownloadModalProps> = ({ isOpen, onClose, onDownload }) => {
  const [filePath, setFilePath] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    if (isOpen) {
      setFilePath('');
    }
  }, [isOpen]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (filePath.trim()) {
      setIsSubmitting(true);
      onDownload(filePath.trim());
      setIsSubmitting(false);
      onClose();
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/70 backdrop-blur-sm z-500 flex items-center justify-center p-4">
      <div className="bg-card border border-border rounded-lg shadow-xl w-full max-w-md">
        <div className="p-6">
          <h2 className="text-xl font-semibold mb-4 bg-gradient-to-r from-primary to-accent bg-clip-text text-transparent">
            Download File
          </h2>
          
          <form onSubmit={handleSubmit} className="space-y-4">
            <div>
              <label className="block text-sm font-medium mb-2 text-foreground">
                File Path
              </label>
              <input
                type="text"
                value={filePath}
                onChange={(e) => setFilePath(e.target.value)}
                placeholder="Enter file path to download (e.g., /home/user/file.txt)"
                className="w-full px-4 py-3 bg-background border border-primary/30 rounded-lg text-foreground focus:border-primary focus:outline-none focus:ring-2 focus:ring-primary/30 transition-all duration-200 shadow-sm hover:border-primary/50"
                autoFocus
              />
              <p className="text-xs text-muted-foreground mt-1">
                Enter the absolute path of the file you want to download from the terminal session.
              </p>
            </div>
            
            <div className="flex justify-end space-x-3 pt-2">
              <button
                type="button"
                onClick={onClose}
                className="px-4 py-2 bg-background hover:bg-primary/10 border border-border rounded-lg text-sm font-medium transition-colors duration-200"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={isSubmitting || !filePath.trim()}
                className={`px-4 py-2 rounded-lg text-sm font-medium transition-all duration-200 flex items-center space-x-2 ${isSubmitting || !filePath.trim() ? 'opacity-50 cursor-not-allowed bg-primary/50' : 'bg-primary hover:bg-primary/90 hover:shadow-lg hover:shadow-primary/20 text-white'}`}
              >
                {isSubmitting ? (
                  <>
                    <svg className="animate-spin -ml-1 mr-2 h-4 w-4 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                      <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                      <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                    </svg>
                    Downloading...
                  </>
                ) : (
                  <>
                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="text-white">
                      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                      <polyline points="7 10 12 15 17 10" />
                      <line x1="12" y1="15" x2="12" y2="3" />
                    </svg>
                    Download
                  </>
                )}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
};