import { Component, Inject, OnInit } from '@angular/core';
import { MAT_DIALOG_DATA, MatDialogRef } from '@angular/material/dialog';
import { GraphServiceService } from '../service/graph-service.service';
import { FormControl, Validators } from '@angular/forms';
import { SimulationConfig } from '../data/SimulationConfig';

@Component({
  selector: 'app-simulation-configurator',
  templateUrl: './simulation-configurator.component.html',
  styleUrls: ['./simulation-configurator.component.css']
})
export class SimulationConfiguratorComponent implements OnInit {

  graphOptions: string[] = [];
  strategyOptions: string[] = [];

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
    private graphService: GraphServiceService,
    @Inject(MAT_DIALOG_DATA) public data: SimulationConfig
  ) {
    this.tryToLoadInputValues(data);
    this.graphFormControl = new FormControl(this.selectedGraph, [Validators.required]);
    this.graphFormControl.valueChanges
      .subscribe(value => this.selectedGraph = value);
    this.fireSourceFormControl = new FormControl(this.fireSources, [Validators.required, Validators.min(1)]);
    this.fireSourceFormControl.valueChanges
      .subscribe(value => this.fireSources = value);
    this.fireFighterFormControl = new FormControl(this.fireFighters, [Validators.required, Validators.min(0)]);
    this.fireFighterFormControl.valueChanges
      .subscribe(value => this.fireFighters = value);
    this.fireFighterFrequencyFormControl = new FormControl(this.fireFighterFrequency, [Validators.required, Validators.min(1)]);
    this.fireFighterFrequencyFormControl.valueChanges
      .subscribe(value => this.fireFighterFrequency = value);
    this.strategyFormcontrol = new FormControl(this.selectedStrategy, [Validators.required]);
    this.strategyFormcontrol.valueChanges
      .subscribe(value => this.selectedStrategy = value);
  }

  tryToLoadInputValues(data: SimulationConfig): void {
    if (data) {
      this.selectedGraph = data.graph_name;
      this.fireSources = data.num_roots;
      this.fireFighters = data.num_ffs;
      this.fireFighterFrequency = data.strategy_every;
      this.selectedStrategy = data.strategy_name;
    }
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

  isConfirmDisabled() : boolean {
    return this.graphFormControl.invalid || this.strategyFormcontrol.invalid || this.fireSourceFormControl.invalid
      || this.fireFighterFormControl.invalid || this.fireFighterFrequencyFormControl.invalid;
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
