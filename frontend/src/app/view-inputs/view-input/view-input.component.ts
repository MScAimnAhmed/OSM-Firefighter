import { Component, EventEmitter, HostListener, OnInit, Output } from '@angular/core';
import { debounceTime, distinctUntilChanged } from 'rxjs/operators';
import { Subject } from 'rxjs';

@Component({
  selector: 'app-view-input',
  templateUrl: './view-input.component.html',
  styleUrls: ['./view-input.component.css']
})
export class ViewInputComponent implements OnInit {

  currentCoord: Coordinates = {
    lat: 0,
    lon: 0
  };
  maxCoord: Coordinates = {
    lat: 0,
    lon: 0
  };
  minCoord: Coordinates = {
    lat: 0,
    lon: 0
  };
  @Output('onChange') onChange = new EventEmitter<Coordinates>();
  lonSubject: Subject<number>;
  latSubject: Subject<number>;

  constructor() {
  }

  ngOnInit(): void {
    this.lonSubject = new Subject<number>();
    this.lonSubject.pipe(
      debounceTime(100),
      distinctUntilChanged()
    ).subscribe(_ => {
      this.onChange.emit(this.currentCoord);
    });
    this.latSubject = new Subject<number>();
    this.latSubject.pipe(
      debounceTime(100),
      distinctUntilChanged()
    ).subscribe(_ => {
      this.onChange.emit(this.currentCoord);
    });
  }

  @HostListener('window:keydown', ['$event'])
  keyEvent(event: KeyboardEvent) {
    if (event.code == KEY_CODE.DOWN_ARROW) {
      //preventDefault to prevent scrolling with arrowkeys
      event.preventDefault();
      this.moveViewVertically(false);
    } else if (event.code == KEY_CODE.UP_ARROW) {
      event.preventDefault();
      this.moveViewVertically(true);
    } else if (event.code == KEY_CODE.RIGHT_ARROW) {
      event.preventDefault();
      this.moveViewHorizontally(true);
    } else if (event.code == KEY_CODE.LEFT_ARROW) {
      event.preventDefault();
      this.moveViewHorizontally(false);
    }
  }

  moveViewHorizontally(moveRight: boolean) {
    //step size is always 1% of the dif between max and min value
    let stepsize = (this.maxCoord.lon - this.minCoord.lon) / 100;
    if (moveRight) {
      if (this.currentCoord.lon + stepsize <= this.maxCoord.lon) {
        this.currentCoord.lon += stepsize;
      }
    } else {
      if (this.currentCoord.lon - stepsize >= this.minCoord.lon) {
        this.currentCoord.lon -= stepsize;
      }
    }
    this.lonSubject.next(this.currentCoord.lon);
  }

  moveViewVertically(moveUp: boolean) {
    //step size is always 1% of the dif between max and min value
    let stepsize = (this.maxCoord.lat - this.minCoord.lat) / 100;
    if (moveUp) {
      if (this.currentCoord.lat + stepsize <= this.maxCoord.lat) {
        this.currentCoord.lat += stepsize;
      }
    } else {
      if (this.currentCoord.lat - stepsize >= this.minCoord.lat) {
        this.currentCoord.lat -= stepsize;
      }
    }
    this.latSubject.next(this.currentCoord.lat);
  }

  roundForDisplay(input: number): number {
    return Math.round((input) * Math.pow(10, 8)) / Math.pow(10, 8);
  }
}

export class Coordinates {
  lat: number;
  lon: number;
}

export enum KEY_CODE {
  UP_ARROW = 'ArrowUp',
  DOWN_ARROW = 'ArrowDown',
  RIGHT_ARROW = 'ArrowRight',
  LEFT_ARROW = 'ArrowLeft'
}
