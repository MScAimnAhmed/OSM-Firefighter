import { Component, EventEmitter, OnInit, Output } from '@angular/core';
import { FormControl, Validators } from '@angular/forms';
import { debounceTime, distinctUntilChanged } from 'rxjs/operators';

@Component({
  selector: 'app-zoom-input',
  templateUrl: './zoom-input.component.html',
  styleUrls: ['./zoom-input.component.css']
})
export class ZoomInputComponent implements OnInit {

  currentZoom = 1;
  currentZoomFormControl: FormControl;
  @Output('onChange') onChange = new EventEmitter<number>();

  constructor() { }

  ngOnInit(): void {
    this.currentZoomFormControl = new FormControl(this.currentZoom, [Validators.required]);
    this.currentZoomFormControl.valueChanges.pipe(
      debounceTime(1000),
      distinctUntilChanged()
    ).subscribe(_ => {
      this.onChange.emit(this.currentZoom);
    });
  }

  changeZoomBy(value: number) {
    this.currentZoom = Math.round((this.currentZoom + value) * 100) / 100;
  }

}
