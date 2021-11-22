import { AfterViewInit, Component, HostListener, OnInit } from '@angular/core';
import { GraphServiceService } from '../service/graph-service.service';
import { SimulationConfig } from '../data/SimulationConfig';
import { SimulationConfiguratorComponent } from '../simulation-configurator/simulation-configurator.component';
import { MatDialog } from '@angular/material/dialog';
import { FormControl, Validators } from '@angular/forms';
import { debounceTime, distinctUntilChanged } from 'rxjs/operators';

@Component({
  selector: 'app-graph-viewer',
  templateUrl: './graph-viewer.component.html',
  styleUrls: ['./graph-viewer.component.css']
})
export class GraphViewerComponent implements OnInit, AfterViewInit {

  simConfig: SimulationConfig;
  currentTurn = 0;
  currentTurnFormControl: FormControl;
  maxTurn = 0;

  currentLat = 0;
  currentLatFormControl: FormControl;
  currentLon = 0;
  currentLonFormControl: FormControl;

  currentZoom = 1;
  currentZoomFormControl: FormControl;

  refreshing: boolean;
  activeSimulation: boolean;

  thumbnail: any;

  constructor(private graphservice: GraphServiceService,
              private dialog: MatDialog) {
  }

  @HostListener('window:keydown', ['$event'])
  keyEvent(event: KeyboardEvent) {
    if (event.code == KEY_CODE.DOWN_ARROW) {
      //preventDefault to prevent scrolling with arrowkeys
      event.preventDefault();
      this.currentLon--;
    } else if (event.code == KEY_CODE.UP_ARROW) {
      event.preventDefault();
      this.currentLon++;
    } else if (event.code == KEY_CODE.RIGHT_ARROW) {
      event.preventDefault();
      this.currentLat++;
    } else if (event.code == KEY_CODE.LEFT_ARROW) {
      event.preventDefault();
      this.currentLat--;
    }
  }

  ngOnInit(): void {
    this.currentLonFormControl = new FormControl(this.currentLon, [Validators.required]);
    this.currentLonFormControl.valueChanges.pipe(
      debounceTime(1000),
      distinctUntilChanged()
    ).subscribe(_ => {
      if (this.activeSimulation) this.refreshView();
    });
    this.currentLatFormControl = new FormControl(this.currentLat, [Validators.required]);
    this.currentLatFormControl.valueChanges.pipe(
      debounceTime(1000),
      distinctUntilChanged()
    ).subscribe(_ => {
      if (this.activeSimulation) this.refreshView();
    });
    this.currentZoomFormControl = new FormControl(this.currentZoom, [Validators.required]);
    this.currentZoomFormControl.valueChanges.pipe(
      debounceTime(1000),
      distinctUntilChanged()
    ).subscribe(_ => {
      if (this.activeSimulation) this.refreshView();
    });
    this.currentTurnFormControl = new FormControl(this.currentTurn, [Validators.required]);
    this.currentTurnFormControl.valueChanges.pipe(
        debounceTime(1000),
        distinctUntilChanged()
      ).subscribe(_ => {
      if (this.activeSimulation) this.refreshView();
    });
  }

  ngAfterViewInit(): void {
  }

  openSimulationConfigDialog() {

    const dialogRef = this.dialog.open(SimulationConfiguratorComponent, {
      width: '470px'
    });

    dialogRef.afterClosed().subscribe((data: SimulationConfig) => {
      this.simConfig = data;
      this.graphservice.simulate(this.simConfig).subscribe(response => {
        this.activeSimulation = true;
        this.maxTurn = response.end_time;
      });
    });
  }

  public refreshView() {
    this.refreshing = true;
    this.graphservice.refreshView(this.currentTurn, this.currentZoom).subscribe((data: Blob) => {
      this.refreshing = false;
      this.createImageFromBlob(data);
    }, _ => {
      console.log('Could not refresh the View');
      this.refreshing = false;
    });
  }

  createImageFromBlob(image: Blob) {
    let reader = new FileReader();
    reader.addEventListener('load', () => {
      this.thumbnail = reader.result;
    }, false);

    if (image) {
      reader.readAsDataURL(image);
    }
  }

  changeZoomBy(value: number) {
    this.currentZoom = Math.round((this.currentZoom + value) * 100) / 100;
  }
}

export enum KEY_CODE {
  UP_ARROW = 'ArrowUp',
  DOWN_ARROW = 'ArrowDown',
  RIGHT_ARROW = 'ArrowRight',
  LEFT_ARROW = 'ArrowLeft'
}
