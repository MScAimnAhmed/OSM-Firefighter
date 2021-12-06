import { Component, OnInit } from '@angular/core';
import { MatDialogRef } from '@angular/material/dialog';
import { GraphServiceService } from '../service/graph-service.service';
import { FormControl, Validators } from '@angular/forms';
import { SimulationConfig } from '../data/SimulationConfig';

@Component({
  selector: 'app-simulation-configurator',
  templateUrl: './simulation-configurator.component.html',
  styleUrls: ['./simulation-configurator.component.css']
})
export class SimulationConfiguratorComponent implements OnInit {

  graphOptions: string[] = ['someTestGraph.fmi', 'another One'];
  strategyOptions: string[] = ['greedy'];

  graphFormControl: FormControl;
  fireSourceFormControl: FormControl;
  fireFighterFormControl: FormControl;
  fireFighterFrequencyFormControl: FormControl;
  strategyFormcontrol: FormControl;

  selectedGraph = '';
  fireSources = 1;
  fireFighters = 1;
  fireFighterFrequency = 1;
  selectedStrategy = '';

  constructor(
    public dialogRef: MatDialogRef<SimulationConfiguratorComponent, SimulationConfig>,
    private graphService: GraphServiceService
  ) {
    this.graphFormControl = new FormControl(this.selectedGraph, [Validators.required]);
    this.graphFormControl.valueChanges
      .subscribe(value => this.selectedGraph = value);
    this.fireSourceFormControl = new FormControl(this.fireSources, [Validators.required]);
    this.fireSourceFormControl.valueChanges
      .subscribe(value => this.fireSources = value);
    this.fireFighterFormControl = new FormControl(this.fireFighters, [Validators.required]);
    this.fireFighterFormControl.valueChanges
      .subscribe(value => this.fireFighters = value);
    this.fireFighterFrequencyFormControl = new FormControl(this.fireFighterFrequency, [Validators.required]);
    this.fireFighterFrequencyFormControl.valueChanges
      .subscribe(value => this.fireFighterFrequency = value);
    this.strategyFormcontrol = new FormControl(this.selectedStrategy, [Validators.required]);
    this.strategyFormcontrol.valueChanges
      .subscribe(value => this.selectedStrategy = value);
  }

  ngOnInit(): void {
    //retrieve Dropdown Options here
    this.graphService.getGraphs().subscribe(
      data => {
        this.graphOptions = data;
      }
    );
    this.graphService.getStrategies().subscribe(
      data => {
        this.strategyOptions = data;
      }
    )
  }

  cancel() {
    this.dialogRef.close();
  }

  confirm() {
    this.dialogRef.close()
    this.dialogRef.close({
        graph_name: this.selectedGraph,
        strategy_name: this.selectedStrategy,
        num_ffs: this.fireFighters,
        num_roots: this.fireSources,
        strategy_every: this.fireFighterFrequency
      }
    );
  }
}
