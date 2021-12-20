import { Component, Inject } from '@angular/core';
import { MAT_DIALOG_DATA, MatDialogRef } from '@angular/material/dialog';
import { GraphServiceService } from '../service/graph-service.service';
import { FormControl, Validators } from '@angular/forms';
import { SimulationConfig } from '../data/SimulationConfig';
import { GraphData } from '../data/GraphData';

@Component({
  selector: 'app-simulation-configurator',
  templateUrl: './simulation-configurator.component.html',
  styleUrls: ['./simulation-configurator.component.css']
})
export class SimulationConfiguratorComponent {

  graphOptions: GraphData[] = [];
  strategyOptions: string[] = [];

  graphFormControl: FormControl;
  fireSourceFormControl: FormControl;
  fireFighterFormControl: FormControl;
  fireFighterFrequencyFormControl: FormControl;
  strategyFormcontrol: FormControl;

  selectedGraph = GraphData;
  fireSources = 1;
  fireFighters = 1;
  fireFighterFrequency = 1;
  selectedStrategy = '';

  constructor(
    public dialogRef: MatDialogRef<SimulationConfiguratorComponent, SimulationConfig>,
    private graphService: GraphServiceService,
    @Inject(MAT_DIALOG_DATA) public data: SimulationConfig
  ) {
    if (data) {
      this.fireSources = data.num_roots;
      this.fireFighters = data.num_ffs;
      this.fireFighterFrequency = data.strategy_every;
    }
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
    this.graphService.getGraphs().subscribe(
      res => {
        this.graphOptions = res;
        let graph = data ? this.graphOptions.find(graph => graph.name === data.graph_name): undefined;
        if (graph) {
          this.graphFormControl.setValue(graph);
        }
      }
    );
    this.graphService.getStrategies().subscribe(
      res => {
        this.strategyOptions = res;
        if (data) this.strategyFormcontrol.setValue(data.strategy_name);
      }
    );
  }

  cancel() {
    this.dialogRef.close();
  }

  isConfirmDisabled() : boolean {
    return this.graphFormControl.invalid || this.strategyFormcontrol.invalid || this.fireSourceFormControl.invalid
      || this.fireFighterFormControl.invalid || this.fireFighterFrequencyFormControl.invalid;
  }

  getNumberOfNodes(): number {
    return this.graphFormControl.value.num_of_nodes ? this.graphFormControl.value.num_of_nodes : 1;
  }

  confirm() {
    this.dialogRef.close()
    this.dialogRef.close({
        graph_name: this.selectedGraph.name,
        strategy_name: this.selectedStrategy,
        num_ffs: this.fireFighters,
        num_roots: this.fireSources,
        strategy_every: this.fireFighterFrequency
      }
    );
  }
}
