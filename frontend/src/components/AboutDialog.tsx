'use client';

import { useState, useEffect } from 'react';
import { getName, getVersion } from '@tauri-apps/api/app';
import { Button } from '@/components/ui/Button';
import { Card } from '@/components/ui/Card';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/Tabs';
import { ScrollArea } from '@/components/ui/ScrollArea';


interface AboutDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

export default function AboutDialog({ isOpen, onClose }: AboutDialogProps) {
  const [appName, setAppName] = useState('NWN2:EE Save Editor');
  const [appVersion, setAppVersion] = useState('0.1.0');
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    const loadInfo = async () => {
      try {
        const name = await getName();
        const ver = await getVersion();
        setAppName(name);
        setAppVersion(ver);
      } catch (e) {
        console.error('Failed to load app info', e);
      }
    };
    if (isOpen) {
      loadInfo();
    }
  }, [isOpen]);

  const handleCopyVersion = async () => {
    try {
      await navigator.clipboard.writeText(`v${appVersion}`);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy', err);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4 animate-in fade-in duration-200">
      <Card className="w-full max-w-2xl h-[550px] flex flex-col overflow-hidden animate-in zoom-in-95 duration-200 bg-[rgb(var(--color-surface-1))] border border-[rgb(var(--color-surface-border))] shadow-2xl p-0">
        
        <div className="p-4 border-b border-[rgb(var(--color-surface-border))] flex justify-between items-center bg-[rgb(var(--color-surface-2))]">
          <h2 className="text-xl font-bold text-[rgb(var(--color-text-primary))]">About</h2>
          <Button variant="ghost" size="sm" onClick={onClose} className="h-8 w-8 p-0">
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </Button>
        </div>

        <div className="flex-1 overflow-hidden flex flex-col">
          <Tabs defaultValue="info" className="flex-1 flex flex-col">
            <div className="px-4 pt-4">
              <TabsList className="w-full flex bg-transparent p-0 gap-2">
                <TabsTrigger 
                  value="info"
                  className="flex-1 h-10 rounded-md border border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] bg-transparent data-[state=active]:!bg-[rgb(var(--color-primary))] data-[state=active]:!text-white transition-colors hover:bg-[rgb(var(--color-primary))/10]"
                >
                  App Info
                </TabsTrigger>
                <TabsTrigger 
                  value="credits"
                  className="flex-1 h-10 rounded-md border border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] bg-transparent data-[state=active]:!bg-[rgb(var(--color-primary))] data-[state=active]:!text-white transition-colors hover:bg-[rgb(var(--color-primary))/10]"
                >
                  Credits
                </TabsTrigger>
                <TabsTrigger 
                  value="legal"
                  className="flex-1 h-10 rounded-md border border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] bg-transparent data-[state=active]:!bg-[rgb(var(--color-primary))] data-[state=active]:!text-white transition-colors hover:bg-[rgb(var(--color-primary))/10]"
                >
                  Legal
                </TabsTrigger>
              </TabsList>
            </div>

            <div className="flex-1 overflow-hidden p-4">
              <TabsContent value="info" className="h-full mt-0">
                <div className="flex flex-col h-full bg-[rgb(var(--color-surface-2))] rounded-lg p-6 items-center text-center space-y-6 overflow-y-auto">
                    
                    <div className="w-24 h-24 bg-gradient-to-br from-[rgb(var(--color-primary))] to-[rgb(var(--color-primary-600))] rounded-2xl flex items-center justify-center shadow-lg mb-2">
                        <span className="text-white font-bold text-5xl">N</span>
                    </div>

                    <div className="flex flex-col items-center">
                        <h3 className="text-2xl font-bold text-[rgb(var(--color-text-primary))]">{appName}</h3>
                        <button 
                            onClick={handleCopyVersion}
                            className="flex items-center justify-center gap-2 mt-2 text-[rgb(var(--color-text-secondary))] hover:text-[rgb(var(--color-primary))] transition-colors cursor-pointer group"
                            title="Copy Version Info"
                        >
                            <span className="font-mono">v{appVersion}</span>
                            {copied ? (
                                <svg className="w-4 h-4 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                                </svg>
                            ) : (
                                <svg className="w-4 h-4 opacity-50 group-hover:opacity-100 transition-opacity" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                                </svg>
                            )}
                        </button>
                    </div>

                    <div className="text-[rgb(var(--color-text-secondary))] max-w-md">
                        <p>A modern, cross-platform save game editor for Neverwinter Nights 2: Enhanced Edition.</p>
                        <p className="mt-2 text-sm opacity-75">Built with Tauri, FastAPI, and Next.js.</p>
                    </div>

                    <div className="grid grid-cols-2 gap-4 w-full max-w-sm pt-4">
                        <a 
                            href="https://github.com/Micromanner/nwn2-ee-save-editor" 
                            target="_blank" 
                            rel="noreferrer"
                            className="flex items-center justify-center gap-2 p-3 bg-[rgb(var(--color-surface-1))] hover:bg-[rgb(var(--color-surface-3))] rounded-lg border border-[rgb(var(--color-surface-border))] transition-colors group"
                        >
                            <svg className="w-5 h-5 text-[rgb(var(--color-text-primary))]" fill="currentColor" viewBox="0 0 24 24">
                                <path fillRule="evenodd" clipRule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" />
                            </svg>
                            <span className="font-medium text-[rgb(var(--color-text-primary))]">GitHub</span>
                        </a>
                        <a 
                            href="https://github.com/Micromanner/nwn2-ee-save-editor/issues" 
                            target="_blank" 
                            rel="noreferrer"
                            className="flex items-center justify-center gap-2 p-3 bg-[rgb(var(--color-surface-1))] hover:bg-[rgb(var(--color-surface-3))] rounded-lg border border-[rgb(var(--color-surface-border))] transition-colors"
                        >
                            <svg className="w-5 h-5 text-[rgb(var(--color-text-primary))]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                            </svg>
                            <span className="font-medium text-[rgb(var(--color-text-primary))]">Report Bug</span>
                        </a>
                    </div>
                </div>
              </TabsContent>

              <TabsContent value="credits" className="h-full mt-0">
                <ScrollArea className="h-full bg-[rgb(var(--color-surface-2))] rounded-lg p-4">
                  <div className="space-y-6">
                    <section>
                      <h4 className="text-sm font-bold uppercase tracking-wider text-[rgb(var(--color-text-muted))] mb-3 border-b border-[rgb(var(--color-surface-border))] pb-1">Created By</h4>
                      <ul className="space-y-2">
                        <li className="flex justify-between items-center text-sm">
                          <span className="text-[rgb(var(--color-text-primary))] font-medium">Micromanner</span>
                        </li>
                      </ul>
                    </section>
                    
                    <section>
                      <h4 className="text-sm font-bold uppercase tracking-wider text-[rgb(var(--color-text-muted))] mb-3 border-b border-[rgb(var(--color-surface-border))] pb-1">Special Thanks</h4>
                      <div className="space-y-3 text-sm text-[rgb(var(--color-text-secondary))] leading-relaxed">
                        <p>
                            <span className="text-[rgb(var(--color-text-primary))] font-medium">Obsidian Entertainment</span> — For creating the original Neverwinter Nights 2 and its rich, moddable foundation.
                        </p>
                        <p>
                            <span className="text-[rgb(var(--color-text-primary))] font-medium">Aspyr</span> — For the 2025 Enhanced Edition, which inspired the creation of this modern editor.
                        </p>
                        <p>
                            <span className="text-[rgb(var(--color-text-primary))] font-medium">ScripterRon</span> — For the <a href="https://neverwintervault.org/project/nwn2/other/tool/nwn2-character-editor" target="_blank" rel="noreferrer" className="text-[rgb(var(--color-primary))] hover:underline">Java-based Character Editor</a>, which served as a vital technical reference for the file specifications used in this project.
                        </p>
                      </div>
                    </section>


                  </div>
                </ScrollArea>
              </TabsContent>

              <TabsContent value="legal" className="h-full mt-0">
                 <ScrollArea className="h-full bg-[rgb(var(--color-surface-2))] rounded-lg p-4">
                  <div className="space-y-6">
                    <section>
                        <h4 className="text-sm font-bold uppercase tracking-wider text-[rgb(var(--color-text-muted))] mb-3 border-b border-[rgb(var(--color-surface-border))] pb-1">License</h4>
                        <div className="text-sm text-[rgb(var(--color-text-secondary))]">
                             This application is open source software licensed under the <a href="https://github.com/Micromanner/nwn2-ee-save-editor/blob/main/LICENSE" target="_blank" rel="noreferrer" className="text-[rgb(var(--color-primary))] hover:underline">MIT License</a>.
                        </div>
                    </section>
                    
                    <section>
                        <h4 className="text-sm font-bold uppercase tracking-wider text-[rgb(var(--color-text-muted))] mb-3 border-b border-[rgb(var(--color-surface-border))] pb-1">Third Party Notices</h4>
                        <div className="text-sm text-[rgb(var(--color-text-secondary))] leading-relaxed">
                             <p className="mb-2">{appName} is unofficial Fan Content permitted under the Fan Content Policy. Not approved/endorsed by Wizards. Portions of the materials used are property of Wizards of the Coast. ©Wizards of the Coast LLC.</p>
                             <p>This project is not affiliated with or endorsed by Aspyr Media.</p>
                        </div>
                    </section>
                  </div>
                </ScrollArea>
              </TabsContent>
            </div>
          </Tabs>
        </div>

        <div className="p-4 border-t border-[rgb(var(--color-surface-border))] bg-[rgb(var(--color-surface-2))] flex justify-end">
          <Button variant="outline" onClick={onClose}>Close</Button>
        </div>
      </Card>
    </div>
  );
}
