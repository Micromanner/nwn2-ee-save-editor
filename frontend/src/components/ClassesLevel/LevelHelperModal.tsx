import { useState, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { X, AlertCircle, AlertTriangle, ArrowRight, Zap, Award } from 'lucide-react';
import { useSubsystem } from '@/contexts/CharacterContext';
import { cn } from '@/lib/utils';

interface LevelHelperModalProps {
  isOpen: boolean;
  onClose: () => void;
  className: string;
  onNavigate?: (path: string) => void;
}

export default function LevelHelperModal({ isOpen, onClose, className, onNavigate }: LevelHelperModalProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [isVisible, setIsVisible] = useState(false);

  // Get live data from subsystems
  const skillsSubsystem = useSubsystem('skills');
  const abilityScoresSubsystem = useSubsystem('abilityScores');
  const featsSubsystem = useSubsystem('feats');

  // Calculate available points from subsystem data
  const skillPoints = (() => {
    const data = skillsSubsystem.data;
    if (!data) return 0;
    const totalAvailable = data.total_available ?? 0;
    const totalSpent = data.spent_points ?? 0;
    return Math.max(0, totalAvailable - totalSpent);
  })();

  const abilityPoints = (() => {
    const data = abilityScoresSubsystem.data;
    if (!data?.point_summary) return 0;
    return data.point_summary.available ?? 0;
  })();

  // Get feat slots from feats subsystem
  const featSlots = (() => {
    const data = featsSubsystem.data as any;
    if (!data?.point_summary) return 0;
    return data.point_summary.available ?? 0;
  })();

  const hasPendingGains = skillPoints > 0 || abilityPoints > 0 || featSlots > 0;

  useEffect(() => {
    if (isOpen) {
      setIsVisible(true);
      setIsExpanded(false);
    } else {
      setIsVisible(false);
    }
  }, [isOpen]);

  if (!isOpen && !isVisible) return null;

  const handleNavigate = (path: string) => {
    if (onNavigate) {
      onNavigate(path);
    }
  };

  const portalRoot = document.getElementById('portal-root') || document.body;

  return createPortal(
    <div className={cn(
      "fixed bottom-6 right-6 z-50 flex flex-col items-end gap-3",
      className
    )}>
      
      {/* Expanded Content Card */}
      <div className={cn(
        "bg-[rgb(var(--color-surface-1))] rounded-lg overflow-hidden transition-[height,opacity,transform,margin] duration-300 origin-bottom-right",
        isExpanded 
          ? "opacity-100 scale-100 translate-y-0 w-80 mb-2 border border-[rgb(var(--color-border))] shadow-2xl" 
          : "opacity-0 scale-95 translate-y-4 w-80 h-0 p-0 overflow-hidden pointer-events-none border-0 shadow-none"
      )}>
         {/* Internal Card Header */}
         <div className="bg-[rgb(var(--color-surface-2))] p-3 border-b border-[rgb(var(--color-border))] flex items-center justify-between">
           <div className="flex items-center gap-2">
             <span className="font-bold text-sm text-[rgb(var(--color-text-primary))]">
               Pending Allocations
             </span>
           </div>
           <button 
             onClick={onClose}
             className="text-[rgb(var(--color-text-muted))] hover:text-[rgb(var(--color-text-primary))]"
             title="Dismiss"
           >
             <X className="w-4 h-4" />
           </button>
         </div>

         {/* Content */}
         <div className="p-4 space-y-3">
            {hasPendingGains ? (
              <p className="text-xs text-[rgb(var(--color-text-muted))]">
                You have pending gains to allocate:
              </p>
            ) : (
              <p className="text-xs text-[rgb(var(--color-text-muted))]">
                No pending allocations. All points have been spent!
              </p>
            )}

            {/* Skills Row */}
            {skillPoints > 0 && (
              <div className="flex items-center justify-between p-2 bg-[rgb(var(--color-surface-2))] rounded hover:bg-[rgb(var(--color-surface-3))] transition-colors cursor-pointer group" onClick={() => handleNavigate('/skills')}>
                <div className="flex items-center gap-2">
                   <div className="p-1.5 bg-green-500/20 text-green-500 rounded-md">
                     <Zap className="w-4 h-4" />
                   </div>
                   <span className="text-sm font-medium">Skill Points</span>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-xs font-bold text-green-500">{skillPoints} Available</span>
                  <ArrowRight className="w-3 h-3 text-[rgb(var(--color-text-muted))] group-hover:translate-x-0.5 transition-transform" />
                </div>
              </div>
            )}

            {/* Feat Slots Row */}
            {featSlots > 0 && (
              <div className="flex items-center justify-between p-2 bg-[rgb(var(--color-surface-2))] rounded hover:bg-[rgb(var(--color-surface-3))] transition-colors cursor-pointer group" onClick={() => handleNavigate('/feats')}>
                <div className="flex items-center gap-2">
                   <div className="p-1.5 bg-purple-500/20 text-purple-500 rounded-md">
                     <Award className="w-4 h-4" />
                   </div>
                   <span className="text-sm font-medium">Feat Slots</span>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-xs font-bold text-purple-500">{featSlots} Available</span>
                  <ArrowRight className="w-3 h-3 text-[rgb(var(--color-text-muted))] group-hover:translate-x-0.5 transition-transform" />
                </div>
              </div>
            )}

            {/* Ability Score Row */}
            {abilityPoints > 0 && (
              <div className="flex items-center justify-between p-2 bg-[rgb(var(--color-surface-2))] rounded hover:bg-[rgb(var(--color-surface-3))] transition-colors cursor-pointer group" onClick={() => handleNavigate('/abilityScores')}>
                 <div className="flex items-center gap-2">
                    <div className="p-1.5 bg-yellow-500/20 text-yellow-500 rounded-md">
                      <AlertCircle className="w-4 h-4" />
                    </div>
                    <span className="text-sm font-medium">Ability Score Increase</span>
                 </div>
                 <div className="flex items-center gap-2">
                   <span className="text-xs font-bold text-yellow-500">{abilityPoints} Available</span>
                   <ArrowRight className="w-3 h-3 text-[rgb(var(--color-text-muted))] group-hover:translate-x-0.5 transition-transform" />
                 </div>
              </div>
            )}

            {/* TODO: Add Spells row when we have spell slot tracking */}
         </div>
      </div>

      {/* Trigger Button (Floating Action Button style) */}
      {hasPendingGains && (
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          className={cn(
            "relative flex items-center justify-center w-12 h-12 rounded-full shadow-lg transition-all duration-300 hover:scale-105 active:scale-95 group z-50",
            isExpanded 
               ? "bg-[rgb(var(--color-surface-3))] text-[rgb(var(--color-text-primary))]" 
               : "bg-blue-600 text-white animate-bounce-subtle"
          )}
          title={isExpanded ? "Close Helper" : "Pending Allocations"}
        >
           {isExpanded ? (
             <X className="w-6 h-6" />
           ) : (
             <>
               <AlertTriangle className="w-6 h-6" />

               {/* Pulse effect ring */}
               <span className="absolute inset-0 rounded-full border-2 border-blue-400 opacity-75 animate-ping-slow"></span>
             </>
           )}
        </button>
      )}

    </div>,
    portalRoot
  );
}
