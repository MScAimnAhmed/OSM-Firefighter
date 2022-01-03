import { ComponentFixture, TestBed } from '@angular/core/testing';

import { ZoomInputComponent } from './zoom-input.component';

describe('ZoomInputComponent', () => {
  let component: ZoomInputComponent;
  let fixture: ComponentFixture<ZoomInputComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [ ZoomInputComponent ]
    })
    .compileComponents();
  });

  beforeEach(() => {
    fixture = TestBed.createComponent(ZoomInputComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
