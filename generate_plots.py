#!/usr/bin/env python3
"""
Generate plots for HQApriori experiments.

This script creates visualizations for:
1. Parameter Sensitivity Analysis (support and confidence thresholds)
2. Record Scalability Testing
3. Attribute Scalability Testing
4. Support Threshold Sensitivity (scatter plot)

Plots are organized in folders matching the experiment structure,
with separate folders for instacart_prior and instacart_train.

Note: All experiment results are averages over multiple runs (N=5 by default).
"""

import json
import os
import glob
from pathlib import Path
import matplotlib.pyplot as plt
import numpy as np
from typing import Dict, List, Tuple, Optional

# Set style (try different style names for compatibility)
try:
    plt.style.use('seaborn-v0_8-darkgrid')
except OSError:
    try:
        plt.style.use('seaborn-darkgrid')
    except OSError:
        plt.style.use('ggplot')
plt.rcParams['figure.figsize'] = (12, 8)
plt.rcParams['font.size'] = 10


def load_json(filepath: str) -> dict:
    """Load JSON file."""
    with open(filepath, 'r') as f:
        return json.load(f)


def ensure_dir(path: str):
    """Ensure directory exists."""
    os.makedirs(path, exist_ok=True)


def plot_parameter_sensitivity_support(dataset_name: str, base_dir: str = "experiments"):
    """
    Plot 1: Support thresholds analysis
    - 3 subplots (one for each level)
    - Each subplot: bar plot with 5 groups (per threshold), 2 bars per group
      (num rules found on level 3, execution time)
    - Use only confidence = 0.2
    
    Note: Even when testing level 1 or 2, num_rules represents rules found at level 3,
    since level 3 is the only level with gen_rules=true in these experiments.
    """
    print(f"Generating support thresholds plot for {dataset_name}...")
    
    # Define support thresholds for each level
    level_supports = {
        1: [0.3, 0.4, 0.5, 0.6, 0.7],
        2: [0.04, 0.06, 0.08, 0.10, 0.12],
        3: [0.001, 0.003, 0.005, 0.007, 0.01]
    }
    
    confidence = 0.2
    
    fig, axes = plt.subplots(3, 1, figsize=(14, 12))
    fig.suptitle(f'Parameter Sensitivity: Support Thresholds (Confidence={confidence})\n{dataset_name}', 
                 fontsize=14, fontweight='bold')
    
    for level_idx, level in enumerate([1, 2, 3]):
        ax = axes[level_idx]
        supports = level_supports[level]
        num_rules = []
        exec_times = []
        
        for support in supports:
            # Find matching file
            pattern = f"{base_dir}/parameter_sensitivity/{dataset_name}_level{level}_s{support}_c{confidence}.json"
            files = glob.glob(pattern)
            
            if files:
                try:
                    data = load_json(files[0])
                    num_rules.append(data['results']['num_rules'])
                    exec_times.append(data['results']['total_time_ms'])
                except (KeyError, json.JSONDecodeError) as e:
                    print(f"    Warning: Error reading {files[0]}: {e}")
                    num_rules.append(0)
                    exec_times.append(0)
            else:
                num_rules.append(0)
                exec_times.append(0)
        
        # Create bar plot
        x = np.arange(len(supports))
        width = 0.35
        
        # Normalize execution time for better visualization (scale to match rule count range)
        max_rules = max(num_rules) if num_rules else 1
        max_time = max(exec_times) if exec_times else 1
        scale_factor = max_rules / max_time if max_time > 0 else 1
        scaled_times = [t * scale_factor for t in exec_times]
        
        bars1 = ax.bar(x - width/2, num_rules, width, label='Number of Rules (Level 3)', 
                       color='steelblue', alpha=0.8)
        bars2 = ax.bar(x + width/2, scaled_times, width, label='Execution Time', 
                       color='coral', alpha=0.8)
        
        # Add value labels on bars
        for bars in [bars1, bars2]:
            for bar in bars:
                height = bar.get_height()
                if height > 0:
                    ax.text(bar.get_x() + bar.get_width()/2., height,
                           f'{int(height)}' if bars == bars1 else f'{int(height/scale_factor)}ms',
                           ha='center', va='bottom', fontsize=8)
        
        ax.set_xlabel('Support Threshold', fontweight='bold')
        ax.set_ylabel('Count / Time (ms)', fontweight='bold')
        ax.set_title(f'Level {level}', fontweight='bold')
        ax.set_xticks(x)
        ax.set_xticklabels([f'{s:.3f}' for s in supports])
        ax.grid(True, alpha=0.3, axis='y')
        ax.set_ylim(bottom=0)  # Ensure bars start at 0
        
        # Add secondary y-axis for actual execution time
        ax2 = ax.twinx()
        ax2.set_ylabel('Execution Time (ms)', fontweight='bold', color='coral')
        ax2.tick_params(axis='y', labelcolor='coral')
        ax2.set_ylim(bottom=0)  # Ensure secondary axis starts at 0
        # Set y-axis limits based on exec_times
        if exec_times:
            ax2.set_ylim(bottom=0, top=max(exec_times) * 1.1)
        
        # Position legend outside plot area to avoid overlapping bars
        ax.legend(loc='upper left', bbox_to_anchor=(1.15, 1), framealpha=0.9)
    
    plt.tight_layout(rect=[0, 0, 0.85, 1])  # Leave space on right for legend
    
    # Save plot
    output_dir = f"{base_dir}/plots/{dataset_name}/parameter_sensitivity"
    ensure_dir(output_dir)
    plt.savefig(f"{output_dir}/support_thresholds.png", dpi=300, bbox_inches='tight')
    plt.close()
    print(f"  Saved to {output_dir}/support_thresholds.png")


def plot_parameter_sensitivity_confidence(dataset_name: str, base_dir: str = "experiments"):
    """
    Plot 2: Confidence thresholds analysis
    - Single bar plot
    - Take 5 runs, all levels with Default Support values
    - Similar structure to support plot
    """
    print(f"Generating confidence thresholds plot for {dataset_name}...")
    
    # Default support values
    default_supports = {1: 0.5, 2: 0.08, 3: 0.005}
    confidences = [0.1, 0.15, 0.2, 0.25, 0.3]
    
    # We'll use level 3 results since that's where rules are generated
    level = 3
    support = default_supports[level]
    
    num_rules = []
    exec_times = []
    
    for confidence in confidences:
        # Find matching file - updated to match new filename pattern from experiments.rs
        pattern = f"{base_dir}/parameter_sensitivity/{dataset_name}_confidence_c{confidence}.json"
        files = glob.glob(pattern)
        
        if files:
            data = load_json(files[0])
            num_rules.append(data['results']['num_rules'])
            exec_times.append(data['results']['total_time_ms'])
        else:
            num_rules.append(0)
            exec_times.append(0)
    
    fig, ax = plt.subplots(figsize=(12, 6))
    fig.suptitle(f'Parameter Sensitivity: Confidence Thresholds\n'
                 f'{dataset_name} (Level 3, Support={support})', 
                 fontsize=14, fontweight='bold')
    
    x = np.arange(len(confidences))
    width = 0.35
    
    # Normalize execution time for better visualization
    max_rules = max(num_rules) if num_rules else 1
    max_time = max(exec_times) if exec_times else 1
    scale_factor = max_rules / max_time if max_time > 0 else 1
    scaled_times = [t * scale_factor for t in exec_times]
    
    bars1 = ax.bar(x - width/2, num_rules, width, label='Number of Rules', 
                   color='steelblue', alpha=0.8)
    bars2 = ax.bar(x + width/2, scaled_times, width, 
                   label='Execution Time', 
                   color='coral', alpha=0.8)
    
    # Add value labels on bars
    for bars in [bars1, bars2]:
        for bar in bars:
            height = bar.get_height()
            if height > 0:
                ax.text(bar.get_x() + bar.get_width()/2., height,
                       f'{int(height)}' if bars == bars1 else f'{int(height/scale_factor)}ms',
                       ha='center', va='bottom', fontsize=9)
    
    ax.set_xlabel('Confidence Threshold', fontweight='bold')
    ax.set_ylabel('Count / Time (ms)', fontweight='bold')
    ax.set_xticks(x)
    ax.set_xticklabels([f'{c:.2f}' for c in confidences])
    ax.grid(True, alpha=0.3, axis='y')
    ax.set_ylim(bottom=0)  # Ensure bars start at 0
    
    # Add secondary y-axis for actual execution time
    ax2 = ax.twinx()
    ax2.set_ylabel('Execution Time (ms)', fontweight='bold', color='coral')
    ax2.tick_params(axis='y', labelcolor='coral')
    ax2.set_ylim(bottom=0)  # Ensure secondary axis starts at 0
    # Set y-axis limits based on exec_times
    if exec_times:
        ax2.set_ylim(bottom=0, top=max(exec_times) * 1.1)
    
    # Position legend outside plot area to avoid overlapping bars
    ax.legend(loc='upper left', bbox_to_anchor=(1.15, 1), framealpha=0.9)
    
    plt.tight_layout(rect=[0, 0, 0.85, 1])  # Leave space on right for legend
    
    # Save plot
    output_dir = f"{base_dir}/plots/{dataset_name}/parameter_sensitivity"
    ensure_dir(output_dir)
    plt.savefig(f"{output_dir}/confidence_thresholds.png", dpi=300, bbox_inches='tight')
    plt.close()
    print(f"  Saved to {output_dir}/confidence_thresholds.png")


def plot_record_scalability(dataset_name: str, base_dir: str = "experiments"):
    """
    Plot 2.1: Record Scalability
    - Scatter plot with trend line: execution time vs multiplier
    """
    print(f"Generating record scalability plot for {dataset_name}...")
    
    multipliers = [1, 2, 4, 8]
    exec_times = []
    
    for mult in multipliers:
        pattern = f"{base_dir}/scalability_records/{dataset_name}_mult{mult}.json"
        files = glob.glob(pattern)
        
        if files:
            data = load_json(files[0])
            exec_times.append(data['results']['total_time_ms'])
        else:
            exec_times.append(0)
    
    fig, ax = plt.subplots(figsize=(8, 6))
    fig.suptitle(f'Record Scalability: {dataset_name}', fontsize=14, fontweight='bold')
    
    # Filter out zero values for trend line calculation
    valid_data = [(m, t) for m, t in zip(multipliers, exec_times) if t > 0]
    if len(valid_data) > 1:
        mults_valid, times_valid = zip(*valid_data)
        
        # Scatter plot
        ax.scatter(multipliers, exec_times, s=100, color='steelblue', alpha=0.7, 
                   edgecolors='black', linewidth=1.5, zorder=3)
        
        # Add trend line (only if we have valid data)
        if len(mults_valid) > 1:
            z = np.polyfit(mults_valid, times_valid, 1)
            p = np.poly1d(z)
            x_trend = np.linspace(min(mults_valid), max(mults_valid), 100)
            ax.plot(x_trend, p(x_trend), '--', color='gray', linewidth=2, alpha=0.7, zorder=2)
            
            # Calculate average time growth factor
            # For 2x multiplier increase, how many times does time increase?
            time_at_mult1 = p(1.0)
            time_at_mult2 = p(2.0)
            growth_factor = time_at_mult2 / time_at_mult1 if time_at_mult1 > 0 else 0
            
            info_text = f'Podwojenie problemu → {growth_factor:.2f}× więcej czasu'
            ax.text(0.05, 0.95, info_text, 
                   transform=ax.transAxes, fontsize=10, verticalalignment='top',
                   bbox=dict(boxstyle='round', facecolor='wheat', alpha=0.5))
    else:
        # Just plot points if not enough data for trend
        ax.scatter(multipliers, exec_times, s=100, color='steelblue', alpha=0.7, 
                   edgecolors='black', linewidth=1.5)
    
    ax.set_xlabel('Multiplier', fontweight='bold')
    ax.set_ylabel('Execution Time (ms)', fontweight='bold')
    ax.grid(True, alpha=0.3)
    ax.set_xticks(multipliers)
    
    # Add value labels
    for mult, time in zip(multipliers, exec_times):
        if time > 0:
            ax.text(mult, time, f'{int(time)}ms', ha='center', va='bottom', fontsize=9)
    
    plt.tight_layout()
    
    # Save plot
    output_dir = f"{base_dir}/plots/{dataset_name}/scalability_records"
    ensure_dir(output_dir)
    plt.savefig(f"{output_dir}/record_scalability.png", dpi=300, bbox_inches='tight')
    plt.close()
    print(f"  Saved to {output_dir}/record_scalability.png")


def plot_attribute_scalability(dataset_name: str, base_dir: str = "experiments"):
    """
    Plot 2.2: Attribute Scalability
    - Scatter plot with trend line: execution time vs attribute multiplier
    """
    print(f"Generating attribute scalability plot for {dataset_name}...")
    
    multipliers = [1, 1.25, 1.5, 1.75, 2]
    exec_times = []
    
    for mult in multipliers:
        pattern = f"{base_dir}/scalability_attributes/{dataset_name}_mult{mult}.json"
        files = glob.glob(pattern)
        
        if files:
            data = load_json(files[0])
            exec_times.append(data['results']['total_time_ms'])
        else:
            exec_times.append(0)
    
    fig, ax = plt.subplots(figsize=(8, 6))
    fig.suptitle(f'Attribute Scalability: {dataset_name}', fontsize=14, fontweight='bold')
    
    # Filter out zero values for trend line calculation
    valid_data = [(m, t) for m, t in zip(multipliers, exec_times) if t > 0]
    if len(valid_data) > 1:
        mults_valid, times_valid = zip(*valid_data)
        
        # Scatter plot
        ax.scatter(multipliers, exec_times, s=100, color='steelblue', alpha=0.7, 
                   edgecolors='black', linewidth=1.5, zorder=3)
        
        # Add trend line (only if we have valid data)
        if len(mults_valid) > 1:
            z = np.polyfit(mults_valid, times_valid, 1)
            p = np.poly1d(z)
            x_trend = np.linspace(min(mults_valid), max(mults_valid), 100)
            ax.plot(x_trend, p(x_trend), '--', color='gray', linewidth=2, alpha=0.7, zorder=2)
            
            # Calculate average time growth factor
            # For 2x multiplier increase, how many times does time increase?
            time_at_mult1 = p(1.0)
            time_at_mult2 = p(2.0)
            growth_factor = time_at_mult2 / time_at_mult1 if time_at_mult1 > 0 else 0
            
            info_text = f'Podwojenie problemu → {growth_factor:.2f}× więcej czasu'
            ax.text(0.05, 0.95, info_text, 
                   transform=ax.transAxes, fontsize=10, verticalalignment='top',
                   bbox=dict(boxstyle='round', facecolor='wheat', alpha=0.5))
    else:
        # Just plot points if not enough data for trend
        ax.scatter(multipliers, exec_times, s=100, color='steelblue', alpha=0.7, 
                   edgecolors='black', linewidth=1.5)
    
    ax.set_xlabel('Attribute Multiplier', fontweight='bold')
    ax.set_ylabel('Execution Time (ms)', fontweight='bold')
    ax.grid(True, alpha=0.3)
    ax.set_xticks(multipliers)
    
    # Add value labels
    for mult, time in zip(multipliers, exec_times):
        if time > 0:
            ax.text(mult, time, f'{int(time)}ms', ha='center', va='bottom', fontsize=9)
    
    plt.tight_layout()
    
    # Save plot
    output_dir = f"{base_dir}/plots/{dataset_name}/scalability_attributes"
    ensure_dir(output_dir)
    plt.savefig(f"{output_dir}/attribute_scalability.png", dpi=300, bbox_inches='tight')
    plt.close()
    print(f"  Saved to {output_dir}/attribute_scalability.png")


def plot_support_threshold_sensitivity(dataset_name: str, base_dir: str = "experiments"):
    """
    Plot 3.2: Support Threshold Sensitivity
    - Scatter plot: num_rules vs execution time
    """
    print(f"Generating support threshold sensitivity plot for {dataset_name}...")
    
    pattern = f"{base_dir}/support_threshold_sensitivity/{dataset_name}.json"
    files = glob.glob(pattern)
    
    if not files:
        print(f"  Warning: No data file found for {dataset_name}")
        return
    
    data = load_json(files[0])
    iterations = data['results']
    
    times = []
    num_rules = []
    level_1_supports = []
    level_2_supports = []
    iteration_numbers = []
    
    # Separate regular points from special point (iteration=0, level_1=0, level_2=0)
    special_times = []
    special_rules = []
    
    for it in iterations:
        iter_num = it.get('iteration', 0)
        l1_sup = it.get('level_1_support', 0)
        l2_sup = it.get('level_2_support', 0)
        
        # Check if this is the special point
        if iter_num == 0 and l1_sup == 0 and l2_sup == 0:
            special_times.append(it['total_time_ms'])
            special_rules.append(it['num_rules'])
        else:
            times.append(it['total_time_ms'])
            num_rules.append(it['num_rules'])
            level_1_supports.append(l1_sup)
            level_2_supports.append(l2_sup)
            iteration_numbers.append(iter_num)
    
    fig, ax = plt.subplots(figsize=(10, 7))
    fig.suptitle(f'Support Threshold Sensitivity: {dataset_name}\n'
                 f'Number of Rules vs Execution Time', 
                 fontsize=14, fontweight='bold')
    
    # Create scatter plot with color based on level 1 support (regular points)
    if times and num_rules:
        scatter = ax.scatter(times, num_rules, c=level_1_supports, 
                            s=50, alpha=0.6, cmap='viridis', edgecolors='black', linewidth=0.5)
        
        # Add colorbar
        cbar = plt.colorbar(scatter, ax=ax)
        cbar.set_label('Level 1 Support', fontweight='bold')
    
    # Plot special point in red if it exists
    if special_times and special_rules:
        ax.scatter(special_times, special_rules, s=150, color='red', 
                  alpha=0.8, edgecolors='darkred', linewidth=2, zorder=5,
                  label='Iteration 0 (L1=0, L2=0)')
        ax.legend(loc='best')
    
    ax.set_xlabel('Execution Time (ms)', fontweight='bold')
    ax.set_ylabel('Number of Rules', fontweight='bold')
    ax.grid(True, alpha=0.3)
    
    plt.tight_layout()
    
    # Save plot
    output_dir = f"{base_dir}/plots/{dataset_name}/support_threshold_sensitivity"
    ensure_dir(output_dir)
    plt.savefig(f"{output_dir}/support_threshold_sensitivity.png", dpi=300, bbox_inches='tight')
    plt.close()
    print(f"  Saved to {output_dir}/support_threshold_sensitivity.png")


def main():
    """Generate all plots for both datasets."""
    print("=" * 60)
    print("HQApriori Plot Generation")
    print("=" * 60)
    
    datasets = ['instacart_train', 'instacart_prior']
    
    for dataset_name in datasets:
        print(f"\n{'='*60}")
        print(f"Processing dataset: {dataset_name}")
        print(f"{'='*60}\n")
        
        # Experiment 1: Parameter Sensitivity
        try:
            plot_parameter_sensitivity_support(dataset_name)
            plot_parameter_sensitivity_confidence(dataset_name)
        except Exception as e:
            print(f"  Error in parameter sensitivity plots: {e}")
        
        # Experiment 2.1: Record Scalability
        try:
            plot_record_scalability(dataset_name)
        except Exception as e:
            print(f"  Error in record scalability plot: {e}")
        
        # Experiment 2.2: Attribute Scalability
        try:
            plot_attribute_scalability(dataset_name)
        except Exception as e:
            print(f"  Error in attribute scalability plot: {e}")
        
        # Experiment 3.2: Support Threshold Sensitivity
        try:
            plot_support_threshold_sensitivity(dataset_name)
        except Exception as e:
            print(f"  Error in support threshold sensitivity plot: {e}")
    
    print(f"\n{'='*60}")
    print("Plot generation completed!")
    print(f"{'='*60}")
    print("\nPlots saved to: experiments/plots/<dataset_name>/<experiment_type>/")


if __name__ == "__main__":
    main()
